//! Periodic, deterministic labour policy shared by every airline. No morale work
//! occurs in quotes: dispatch cursors carry only a dated departure hold.
use crate::{
    game::Game,
    staff::{self, Role},
};
use serde::{Deserialize, Serialize};

const DAY: u64 = 86_400;
#[derive(Deserialize)]
pub struct Rules {
    pub minimum_pay_percent: u32,
    pub maximum_pay_percent: u32,
    pub market_pay_percent: u32,
    pub neutral_morale: i32,
    pub minimum_target: i32,
    pub maximum_target: i32,
    pub response_per_period: i32,
    pub understaffing_penalty: i32,
    pub missed_payroll_penalty: i32,
    pub restructuring_penalty: i32,
    pub calm_threshold: i32,
    pub resignation_threshold: i32,
    pub strike_threshold: i32,
    pub resignation_bps: u32,
    pub warning_days: u64,
    pub strike_days: u64,
    pub cooldown_days: u64,
    pub ai_distress_pay_percent: u32,
    pub ai_recovery_morale: i32,
}
impl Rules {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..self.market_pay_percent).contains(&self.minimum_pay_percent)
            || self.market_pay_percent != 100
            || self.maximum_pay_percent <= 100
            || self.maximum_pay_percent > 200
            || !(0..self.strike_threshold).contains(&self.minimum_target)
            || !(self.strike_threshold..self.calm_threshold).contains(&self.resignation_threshold)
            || !(self.calm_threshold..=100).contains(&self.neutral_morale)
            || !(self.neutral_morale..=100).contains(&self.maximum_target)
            || !(1..=100).contains(&self.response_per_period)
            || [
                self.understaffing_penalty,
                self.missed_payroll_penalty,
                self.restructuring_penalty,
            ]
            .iter()
            .any(|p| !(0..=100).contains(p))
            || !(1..=10_000).contains(&self.resignation_bps)
            || [self.warning_days, self.strike_days, self.cooldown_days]
                .iter()
                .any(|v| !(1..=3650).contains(v))
            || !(self.minimum_pay_percent..=100).contains(&self.ai_distress_pay_percent)
            || !(self.resignation_threshold..=100).contains(&self.ai_recovery_morale)
        {
            return Err("Ungültige Gehalts- und Moralregeln.".into());
        }
        Ok(())
    }
    pub fn target(&self, pay: u32) -> i32 {
        if pay <= 100 {
            self.minimum_target
                + (self.neutral_morale - self.minimum_target)
                    * (pay - self.minimum_pay_percent) as i32
                    / (100 - self.minimum_pay_percent) as i32
        } else {
            self.neutral_morale
                + (self.maximum_target - self.neutral_morale) * (pay - 100) as i32
                    / (self.maximum_pay_percent - 100) as i32
        }
    }
}
pub fn rules() -> &'static Rules {
    &staff::config().expect("staff config").labour
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub pay_percent: u32,
    pub pending_percent: u32,
    pub morale: i32,
    pub warning_at: Option<u64>,
    pub strike_until: Option<u64>,
    pub cooldown_until: u64,
    /// Last payroll's factors; text is rendered from these facts.
    pub understaffed: bool,
    pub missed_payroll: bool,
    pub restructuring: bool,
    pub resigned: u32,
}
impl Default for Group {
    fn default() -> Self {
        let r = rules();
        Self {
            pay_percent: r.market_pay_percent,
            pending_percent: r.market_pay_percent,
            morale: r.neutral_morale,
            warning_at: None,
            strike_until: None,
            cooldown_until: 0,
            understaffed: false,
            missed_payroll: false,
            restructuring: false,
            resigned: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hold {
    pub index: u64,
    pub from: u64,
    pub until: u64,
}

impl Game {
    pub fn labour_group(&self, role: Role) -> &Group {
        &self.staff.labour[role as usize]
    }
    pub fn pay_salary(&self, role: Role) -> i64 {
        staff::salary(role).unwrap_or(0) * i64::from(self.labour_group(role).pay_percent) / 100
    }
    pub fn pay_preview(&self, role: Role, percent: u32) -> Result<i64, String> {
        if !(rules().minimum_pay_percent..=rules().maximum_pay_percent).contains(&percent) {
            return Err("Gehaltsstufe außerhalb des Bereichs.".into());
        }
        Ok(staff::salary(role)? * i64::from(percent) / 100 * i64::from(self.staff_count(role)))
    }
    pub fn set_pay_level(&mut self, role: Role, percent: u32) -> Result<(), String> {
        self.require_active()?;
        if self.airline.is_none() {
            return Err("Bitte zuerst eine Airline gründen.".into());
        }
        self.pay_preview(role, percent)?;
        let g = &mut self.staff.labour[role as usize];
        g.pending_percent = percent;
        // A credible raise averts the warning immediately; morale still changes
        // only at payroll. A strike already in progress runs its fixed duration.
        if percent > g.pay_percent
            && rules().target(percent) >= rules().calm_threshold
            && g.warning_at.take().is_some()
        {
            self.refresh_labour_holds()?;
        }
        Ok(())
    }
    pub fn next_labour_event(&self) -> Option<u64> {
        let mut next: Option<u64> = None;
        for group in &self.staff.labour {
            if let Some(time) = group.warning_at.or(group.strike_until) {
                next = Some(next.map_or(time, |old| old.min(time)));
            }
        }
        next
    }
    pub(crate) fn migrate_labour(&mut self) {
        self.staff.labour = Default::default();
        self.staff.labour_period = self.clock.elapsed().as_secs()
            / staff::period_seconds().unwrap()
            * staff::period_seconds().unwrap();
    }
    pub(crate) fn close_labour_period(&mut self, due: u64, missed: bool) -> Result<(), String> {
        let r = rules();
        let (pilots, cabin) = self
            .fleet
            .iter()
            .filter_map(|p| staff::needs(&p.model_id).ok())
            .fold((0, 0), |(p, c), n| (p + n.pilots, c + n.cabin_crew));
        let required = [pilots, cabin, self.technician_target()];
        let mut changed = false;
        for role in Role::ALL {
            let count = self.staff_count(role);
            let short = count < required[role as usize]
                || role == Role::Pilot
                    && staff::ratings()
                        .iter()
                        .any(|rating| self.rated_count(rating) < self.rated_busy(rating));
            let g = &mut self.staff.labour[role as usize];
            g.understaffed = short;
            g.missed_payroll = missed;
            g.restructuring = self.resolution.deadline.is_some();
            let target = (r.target(g.pay_percent)
                - if short { r.understaffing_penalty } else { 0 }
                - if missed { r.missed_payroll_penalty } else { 0 }
                - if g.restructuring {
                    r.restructuring_penalty
                } else {
                    0
                })
            .clamp(0, 100);
            g.morale += (target - g.morale).clamp(-r.response_per_period, r.response_per_period);
            g.resigned = if g.morale < r.resignation_threshold {
                (count * r.resignation_bps).div_ceil(10_000)
            } else {
                0
            };
            let resign = g.resigned;
            if count > g.resigned
                && g.morale < r.strike_threshold
                && g.warning_at.is_none()
                && g.strike_until.is_none()
                && due >= g.cooldown_until
            {
                g.warning_at = Some(
                    due.checked_add(r.warning_days * DAY)
                        .ok_or("Zeitüberlauf.")?,
                );
                changed = true;
            }
            // Stable employee ids, including rated pilots and employees reserved
            // by a plan. Existing airborne quotes finish with their original crew.
            for _ in 0..resign {
                if let Some(i) = self.staff.employees.iter().rposition(|e| e.role == role) {
                    self.resign_employee(i)?;
                    changed = true;
                }
            }
        }
        // The period just ended is paid at its old rate. New rates accrue from
        // this boundary, never retroactively from an employee's hire date.
        for role in Role::ALL {
            let g = &mut self.staff.labour[role as usize];
            if g.pay_percent == g.pending_percent {
                continue;
            }
            g.pay_percent = g.pending_percent;
            let salary = staff::salary(role)? * i64::from(g.pay_percent) / 100;
            for e in self
                .staff
                .employees
                .iter_mut()
                .filter(|e| e.role == role && e.salary_eur != salary)
            {
                let period = staff::period_seconds()?;
                e.salary_remainder = ((i128::from(e.salary_eur)
                    * i128::from(due.saturating_sub(e.salary_since.unwrap_or(e.hired_at)))
                    + i128::from(e.salary_remainder))
                    % i128::from(period)) as u64;
                e.salary_eur = salary;
                e.salary_since = Some(due);
                e.paid_until = due.max(e.hired_at);
            }
        }
        self.staff.labour_period = due;
        if changed {
            self.refresh_labour_holds()?;
        }
        Ok(())
    }
    pub(crate) fn apply_labour_events(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        if self.next_labour_event().is_none_or(|t| t > now) {
            return Ok(());
        }
        for g in &mut self.staff.labour {
            if g.strike_until.is_some_and(|t| t <= now) {
                g.strike_until = None;
            }
            if g.warning_at.is_some_and(|t| t <= now) {
                g.warning_at = None;
                if g.morale < rules().strike_threshold {
                    let end = now
                        .checked_add(rules().strike_days * DAY)
                        .ok_or("Zeitüberlauf.")?;
                    g.strike_until = Some(end);
                    g.cooldown_until = end
                        .checked_add(rules().cooldown_days * DAY)
                        .ok_or("Zeitüberlauf.")?;
                }
            }
        }
        self.refresh_labour_holds()
    }
    pub(crate) fn ai_labour_policy(&mut self) -> Result<(), String> {
        let period = staff::period_seconds()?;
        if !self.clock.elapsed().as_secs().is_multiple_of(period)
            || self.staff.labour_period == self.clock.elapsed().as_secs()
            || self.insolvent()
        {
            return Ok(());
        }
        let distressed = self.distressed();
        for role in Role::ALL {
            // Restore market pay for unhappy employees, using the player's
            // command so a credible raise also averts an existing warning.
            let percent =
                if distressed && self.labour_group(role).morale >= rules().ai_recovery_morale {
                    rules().ai_distress_pay_percent
                } else {
                    rules().market_pay_percent
                };
            self.set_pay_level(role, percent)?;
        }
        Ok(())
    }
    pub(crate) fn clear_labour_action(&mut self) {
        for g in &mut self.staff.labour {
            g.warning_at = None;
            g.strike_until = None;
        }
    }
    pub(crate) fn validate_labour(&self) -> Result<(), String> {
        let r = rules();
        let now = self.clock.elapsed().as_secs();
        if self.staff.labour_period > now
            || !self
                .staff
                .labour_period
                .is_multiple_of(staff::period_seconds()?)
            || self.staff.labour.iter().any(|g| {
                !(r.minimum_pay_percent..=r.maximum_pay_percent).contains(&g.pay_percent)
                    || !(r.minimum_pay_percent..=r.maximum_pay_percent).contains(&g.pending_percent)
                    || !(0..=100).contains(&g.morale)
                    || g.warning_at.is_some() && g.strike_until.is_some()
                    || [g.warning_at, g.strike_until]
                        .into_iter()
                        .flatten()
                        .any(|t| t <= now || t > u64::MAX / 2)
            })
        {
            return Err("Ungültige Arbeitskampfdaten.".into());
        }
        Ok(())
    }
    /// Rebuild only on labour events or staffing actions. Normal flight quotes
    /// read one optional dated hold from the existing deterministic cursor.
    /// Scheduled aircraft lacking crew under the same route-order allocation the staffing hold uses:
    /// `(aircraft_id, missing pilots, missing cabin crew)`.
    pub fn crew_gaps(&self) -> Vec<(u32, u32, u32)> {
        let mut pilots: std::collections::BTreeMap<_, _> = staff::ratings()
            .into_iter()
            .map(|r| {
                let n = self.rated_count(&r);
                (r, n)
            })
            .collect();
        let mut cabin = self.staff_count(Role::CabinCrew);
        let mut gaps = Vec::new();
        for route in self.routes.iter().filter(|r| r.plan.is_some()) {
            let Some(id) = route.aircraft_id else {
                continue;
            };
            let Some(plane) = self.fleet.iter().find(|p| p.id == id) else {
                continue;
            };
            let Ok(n) = staff::needs(&plane.model_id) else {
                continue;
            };
            let Some(free) = pilots.get_mut(&n.rating) else {
                continue;
            };
            let (lack_pilots, lack_cabin) = (
                n.pilots.saturating_sub(*free),
                n.cabin_crew.saturating_sub(cabin),
            );
            *free = free.saturating_sub(n.pilots);
            cabin = cabin.saturating_sub(n.cabin_crew);
            if lack_pilots > 0 || lack_cabin > 0 {
                gaps.push((id, lack_pilots, lack_cabin));
            }
        }
        gaps
    }

    pub(crate) fn refresh_labour_holds(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        if self.staff.resignations == 0
            && self
                .staff
                .labour
                .iter()
                .all(|g| g.warning_at.is_none() && g.strike_until.is_none())
            && self.routes.iter().all(|r| {
                r.plan
                    .as_ref()
                    .and_then(|p| p.disruption.as_ref())
                    .is_none_or(|c| c.staff_hold.is_none())
            })
        {
            return Ok(());
        }
        self.initialize_disruptions()?;
        let mut pilots: std::collections::BTreeMap<_, _> = staff::ratings()
            .into_iter()
            .map(|r| {
                let n = self.rated_count(&r);
                (r, n)
            })
            .collect();
        let mut cabin = self.staff_count(Role::CabinCrew);
        for i in 0..self.routes.len() {
            let route = &self.routes[i];
            let Some(plan) = &route.plan else { continue };
            let plane = self
                .fleet
                .iter()
                .find(|p| Some(p.id) == route.aircraft_id)
                .ok_or("Flugzeug fehlt.")?;
            let n = staff::needs(&plane.model_id)?;
            let free = pilots.get_mut(&n.rating).ok_or("Muster fehlt.")?;
            let short = *free < n.pilots || cabin < n.cabin_crew;
            *free = free.saturating_sub(n.pilots);
            cabin = cabin.saturating_sub(n.cabin_crew);
            let mut hold = short.then_some((now, u64::MAX / 4));
            for role in Role::ALL {
                if role == Role::CabinCrew && n.cabin_crew == 0 {
                    continue;
                }
                let g = self.labour_group(role);
                let candidate = g.strike_until.map(|end| (now, end)).or_else(|| {
                    g.warning_at
                        .map(|start| (start, start + rules().strike_days * DAY))
                });
                if let Some((from, until)) = candidate {
                    hold = Some(hold.map_or((from, until), |(a, b)| (a.min(from), b.max(until))));
                }
            }
            let projection = crate::disruptions::project(route, &plane.model_id, now)?;
            let was_airborne = plan.airborne.is_some();
            let c = self.routes[i]
                .plan
                .as_mut()
                .unwrap()
                .disruption
                .as_mut()
                .unwrap();
            // Preserve the exact leg already dispatched while changing later departures.
            if was_airborne {
                c.frozen = projection.active;
            }
            match hold {
                Some((from, until)) => {
                    c.staff_hold = Some(Box::new(Hold {
                        index: c.index + u64::from(was_airborne),
                        from,
                        until,
                    }))
                }
                None => {
                    if let Some(h) = &mut c.staff_hold {
                        if now < h.from {
                            c.staff_hold = None;
                        } else {
                            h.until = h.until.min(now);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
