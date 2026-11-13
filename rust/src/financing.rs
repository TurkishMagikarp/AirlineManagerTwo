//! One amortizing facility, whole EUR, simulation-time payments; no implicit credit.
use crate::{aircraft::AircraftStatus, game::Game};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

const DAY: u64 = 86400;
#[derive(Deserialize)]
pub struct Rules {
    pub maximum_loan_eur: i64,
    pub annual_interest_bps: i64,
    pub repayment_period_days: u64,
    pub term_periods: i64,
    pub low_cash_warning_eur: i64,
    pub distress_grace_days: u64,
    pub restructuring_days: u64,
    pub reschedule_term_periods: i64,
    pub reschedule_fee_bps: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/financing.json"))
        .expect("financing config");
    assert!((1..=1_000_000_000).contains(&r.maximum_loan_eur));
    assert!((0..=10000).contains(&r.annual_interest_bps));
    assert!((1..=365).contains(&r.repayment_period_days));
    assert!((1..=120).contains(&r.term_periods));
    assert!(r.low_cash_warning_eur >= 0);
    assert!((1..=3650).contains(&r.distress_grace_days));
    assert!((1..=3650).contains(&r.restructuring_days));
    assert!((r.term_periods + 1..=240).contains(&r.reschedule_term_periods));
    assert!((0..=10000).contains(&r.reschedule_fee_bps));
    r
});

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Financing {
    pub debt: i64,
    pub installment: i64,
    pub overdue: i64,
    pub next_due: u64,
}

/// Signed cash movement, separate from operating profit and aircraft purchases.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub time: u64,
    pub cash: i64,
    pub principal: i64,
    pub interest: i64,
}

/// Separate from the facility: paying off debt must not erase distress or rescue use.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub distress_since: Option<u64>,
    pub deadline: Option<u64>,
    pub rescheduled: bool,
    pub insolvent_at: Option<u64>,
}

impl Game {
    pub fn insolvent(&self) -> bool {
        self.resolution.insolvent_at.is_some()
    }

    pub fn require_active(&self) -> Result<(), String> {
        if self.insolvent() {
            Err("Insolvent: Unternehmensführung beendet. Dieser Spielstand bleibt einsehbar; kein Neustart in dieser Welt.".into())
        } else {
            Ok(())
        }
    }

    /// Observe cash-changing actions immediately, including while paused. The
    /// terminal transition waits until the complete event has settled.
    pub fn observe_distress(&mut self) {
        if self.insolvent() || self.airline.is_none() {
            return;
        }
        if self.distressed() {
            self.resolution
                .distress_since
                .get_or_insert(self.clock.elapsed().as_secs());
        } else {
            self.resolution.distress_since = None;
            self.resolution.deadline = None;
        }
    }

    /// Called only after all bills at an event have settled. Recovery wins at
    /// the deadline when that event's final balance covers every obligation.
    pub fn apply_resolution(&mut self) {
        if self.insolvent() || self.airline.is_none() {
            return;
        }
        if !self.distressed() {
            self.resolution.distress_since = None;
            self.resolution.deadline = None;
            return;
        }
        let now = self.clock.elapsed().as_secs();
        let since = *self.resolution.distress_since.get_or_insert(now);
        let start = since.saturating_add(RULES.distress_grace_days * DAY);
        if now >= start {
            let deadline = start.saturating_add(RULES.restructuring_days * DAY);
            self.resolution.deadline = Some(deadline);
            if now >= deadline {
                self.resolution.insolvent_at = Some(deadline);
                self.clock.set_running(false);
            }
        }
    }

    pub fn reschedule_fee(&self) -> i64 {
        ((i128::from(self.financing.debt) * i128::from(RULES.reschedule_fee_bps) + 9999) / 10000)
            as i64
    }

    pub fn can_reschedule(&self) -> bool {
        !self.insolvent()
            && self.resolution.deadline.is_some()
            && !self.resolution.rescheduled
            && self.financing.debt > 0
    }

    pub fn reschedule_loan(&mut self) -> Result<(), String> {
        if !self.can_reschedule() {
            return Err(
                "Umschuldung nur einmal während einer Restrukturierung mit Restschuld möglich."
                    .into(),
            );
        }
        let fee = self.reschedule_fee();
        let due = self
            .clock
            .elapsed()
            .as_secs()
            .checked_add(RULES.repayment_period_days * DAY)
            .ok_or("Zeitüberlauf.")?;
        // Fee is a financing expense, using the same reconciled cash journal as interest.
        self.post_financing(-fee, 0, fee)?;
        self.financing.installment = (self.financing.debt + RULES.reschedule_term_periods - 1)
            / RULES.reschedule_term_periods;
        self.financing.overdue = 0;
        self.financing.next_due = due;
        self.resolution.rescheduled = true;
        self.apply_resolution();
        Ok(())
    }

    pub fn cash(&self) -> i64 {
        self.airline.as_ref().map_or(0, |a| a.capital_eur)
    }

    pub fn distressed(&self) -> bool {
        self.cash() < 0 || self.financing.overdue > 0
    }

    pub fn financing_status(&self) -> &'static str {
        if self.insolvent() {
            "INSOLVENT — Unternehmensführung beendet. Spielstand bleibt einsehbar; kein Neustart in dieser Welt."
        } else if self.resolution.deadline.is_some() {
            "Restrukturierung: Frist beachten. Kredit einmalig umschulden oder Flugzeuge verkaufen / zurückgeben."
        } else if self.distressed() {
            "Finanznot: Verpflichtungen offen. Expansion und neue Flüge gesperrt; laufende Flüge enden regulär. Kosten senken oder Kredit aufnehmen."
        } else if self.cash() < RULES.low_cash_warning_eur {
            "Warnung: geringe Liquidität. Gehälter, Wartung und Kreditraten einplanen."
        } else {
            "Zahlungsfähig"
        }
    }

    pub fn require_solvent(&self) -> Result<(), String> {
        self.require_active()?;
        if self.distressed() {
            Err(self.financing_status().into())
        } else {
            Ok(())
        }
    }

    pub fn loan_interest(&self) -> i64 {
        // Simple annual rate, actual configured period / 365, rounded up once.
        let n = i128::from(self.financing.debt)
            * i128::from(RULES.annual_interest_bps)
            * i128::from(RULES.repayment_period_days);
        ((n + 3_649_999) / 3_650_000) as i64
    }

    pub fn loan_payment(&self) -> i64 {
        self.loan_interest()
            + self.financing.debt.min(
                self.financing
                    .installment
                    .saturating_add(self.financing.overdue),
            )
    }

    fn post_financing(&mut self, cash: i64, principal: i64, interest: i64) -> Result<(), String> {
        let balance = self.cash().checked_add(cash).ok_or("Finanzüberlauf.")?;
        let debt = self
            .financing
            .debt
            .checked_add(principal)
            .ok_or("Finanzüberlauf.")?;
        let a = self
            .airline
            .as_mut()
            .ok_or("Bitte zuerst eine Airline gründen.")?;
        let h = self.history.as_mut().ok_or("Finanzhistorie fehlt.")?;
        a.capital_eur = balance;
        self.financing.debt = debt;
        h.financing.push(Entry {
            time: self.clock.elapsed().as_secs(),
            cash,
            principal,
            interest,
        });
        Ok(())
    }

    pub fn borrow(&mut self, amount: i64) -> Result<(), String> {
        self.require_active()?;
        if amount <= 0 || amount > RULES.maximum_loan_eur || self.financing.debt != 0 {
            return Err(format!(
                "Ein Kredit bis {} EUR; erst vollständig tilgen, bevor ein neuer Kredit möglich ist.",
                RULES.maximum_loan_eur
            ));
        }
        // Negative legacy/operating balance may be rescued, but not an insolvent facility.
        if self.cash().checked_add(amount).is_none_or(|v| v < 0) {
            return Err("Kredit muss die offenen Verpflichtungen decken.".into());
        }
        let due = self
            .clock
            .elapsed()
            .as_secs()
            .checked_add(RULES.repayment_period_days * DAY)
            .ok_or("Zeitüberlauf.")?;
        self.post_financing(amount, amount, 0)?;
        self.financing.installment = (amount + RULES.term_periods - 1) / RULES.term_periods;
        self.financing.next_due = due;
        self.observe_distress();
        Ok(())
    }

    pub fn repay(&mut self, amount: i64) -> Result<(), String> {
        self.require_active()?;
        if amount <= 0 || amount > self.financing.debt || amount > self.cash() {
            return Err(
                "Tilgung benötigt verfügbares Guthaben und darf Restschuld nicht übersteigen."
                    .into(),
            );
        }
        self.post_financing(-amount, -amount, 0)?;
        self.financing.overdue = (self.financing.overdue - amount).max(0);
        if self.financing.debt == 0 {
            self.financing = Financing::default();
        }
        self.observe_distress();
        Ok(())
    }

    pub fn next_financing_event(&self) -> Option<u64> {
        self.airline.as_ref()?;
        if self.insolvent() {
            return None;
        }
        let now = self.clock.elapsed().as_secs();
        let mut next = self
            .next_maintenance_event()
            .into_iter()
            .chain(self.next_payroll_event())
            .chain(self.next_lease_event())
            .min()
            .unwrap_or(u64::MAX);
        if self.financing.debt > 0 {
            next = next.min(self.financing.next_due.max(now));
        }
        if self.distressed() {
            next = next.min(match self.resolution.distress_since {
                None => now,
                Some(since) => self
                    .resolution
                    .deadline
                    .unwrap_or_else(|| since.saturating_add(RULES.distress_grace_days * DAY))
                    .max(now),
            });
        } else if self.resolution.distress_since.is_some() {
            next = now;
        }
        // Batch safely while cash covers even the worst possible flight costs
        // until the next mandatory bill. Otherwise settle at each landing.
        // This keeps centuries of funded archive tests compact without allowing
        // a large step to fly past the instant cash runs out.
        let mut worst_costs = 0i128;
        let mut first_landing = u64::MAX;
        for r in self.routes.iter().filter(|r| r.plan.is_some()) {
            if let Some(p) = self.fleet.iter().find(|p| Some(p.id) == r.aircraft_id)
                && let Ok(m) = crate::operations::movement(r, &p.model_id, now)
            {
                let landing = if m.landing > now {
                    m.landing
                } else {
                    m.landing
                        .saturating_add(r.plan.as_ref().unwrap().interval_seconds)
                };
                first_landing = first_landing.min(landing);
                let mut free = r.clone();
                free.ticket_eur = 0;
                let ceiling = crate::economy::quote(&free, &p.model_id)
                    .map_or(i64::MAX, |t| t.costs())
                    .max(
                        r.plan
                            .as_ref()
                            .unwrap()
                            .airborne
                            .as_ref()
                            .map_or(0, |f| f.totals.costs()),
                    );
                let count =
                    next.saturating_sub(now) / r.plan.as_ref().unwrap().interval_seconds + 1;
                worst_costs = worst_costs.saturating_add(i128::from(ceiling) * i128::from(count));
            }
        }
        if i128::from(self.cash()) < worst_costs || self.financing.overdue > 0 {
            next = next.min(first_landing);
        }
        // Distress never erases airborne quotes: suspend only after ground time.
        if self.distressed() {
            for r in self.routes.iter().filter(|r| r.plan.is_some()) {
                if let Some(p) = self.fleet.iter().find(|p| Some(p.id) == r.aircraft_id)
                    && let Ok(m) = crate::operations::movement(r, &p.model_id, now)
                {
                    next = next.min(if p.status == AircraftStatus::Parked {
                        now
                    } else {
                        m.ready.max(now + 1)
                    });
                }
            }
        }
        (next != u64::MAX).then_some(next)
    }

    pub fn apply_financing(&mut self) -> Result<(), String> {
        if self.insolvent() {
            return Ok(());
        }
        self.apply_leases()?;
        let now = self.clock.elapsed().as_secs();
        if self.financing.debt > 0 && self.financing.next_due <= now {
            let interest = self.loan_interest();
            self.post_financing(-interest, 0, interest)?;
            self.financing.overdue = self.financing.debt.min(
                self.financing
                    .overdue
                    .checked_add(self.financing.installment)
                    .ok_or("Finanzüberlauf.")?,
            );
            self.financing.next_due = now
                .checked_add(RULES.repayment_period_days * DAY)
                .ok_or("Zeitüberlauf.")?;
        }
        let payment = self.financing.overdue.min(self.cash().max(0));
        if payment > 0 {
            self.repay(payment)?;
        }
        if self.distressed() {
            // At a zero-gap turnaround movement() already describes the next
            // departure. Cancel that departure, not the flight that just landed.
            if now > 0 {
                for i in 0..self.routes.len() {
                    let r = &self.routes[i];
                    if r.plan.is_none() {
                        continue;
                    }
                    if let Some(p) = self.fleet.iter().position(|p| Some(p.id) == r.aircraft_id)
                        && let Ok(m) =
                            crate::operations::movement(r, &self.fleet[p].model_id, now - 1)
                        && m.ready == now
                    {
                        self.fleet[p].status = AircraftStatus::Parked;
                        self.fleet[p].location_icao = m.destination;
                        self.routes[i].plan = None;
                    }
                }
            }
            let ids: Vec<_> = self
                .routes
                .iter()
                .filter(|r| r.plan.is_some())
                .filter(|r| {
                    self.fleet
                        .iter()
                        .any(|p| Some(p.id) == r.aircraft_id && p.status == AircraftStatus::Parked)
                })
                .map(|r| r.id)
                .collect();
            for id in ids {
                self.stop_schedule(id)?;
            }
        }
        self.apply_resolution();
        Ok(())
    }

    pub fn validate_financing(&self) -> Result<(), String> {
        let r = &self.resolution;
        let now = self.clock.elapsed().as_secs();
        if r.distress_since.is_some_and(|s| s > now)
            || r.deadline.is_some_and(|d| {
                r.distress_since.is_none_or(|s| {
                    d != s.saturating_add(
                        (RULES.distress_grace_days + RULES.restructuring_days) * DAY,
                    )
                }) || d.saturating_sub(RULES.restructuring_days * DAY) > now
            })
            || r.insolvent_at.is_some_and(|t| {
                Some(t) != r.deadline || t != now || self.clock.is_running() || !self.distressed()
            })
            || (!self.insolvent() && r.deadline.is_some_and(|d| d <= now))
            || (self.airline.is_none() && *r != Resolution::default())
        {
            return Err("Ungültiger Restrukturierungszustand.".into());
        }
        let f = &self.financing;
        if f.debt < 0
            || f.debt > RULES.maximum_loan_eur
            || f.overdue < 0
            || f.overdue > f.debt
            || (f.debt == 0 && *f != Financing::default())
            || (f.debt > 0
                && (f.installment <= 0
                    || f.installment > RULES.maximum_loan_eur
                    || f.next_due <= self.clock.elapsed().as_secs()))
        {
            return Err("Ungültige Kreditdaten.".into());
        }
        let mut principal = 0i128;
        if let Some(h) = &self.history {
            let mut previous = h.since;
            for e in &h.financing {
                if e.time < previous
                    || e.time > self.clock.elapsed().as_secs()
                    || e.interest < 0
                    || i128::from(e.cash) != i128::from(e.principal) - i128::from(e.interest)
                    || (e.interest > 0 && e.principal != 0)
                {
                    return Err("Ungültiges Kreditjournal.".into());
                }
                principal += i128::from(e.principal);
                if principal < 0 || principal > i128::from(RULES.maximum_loan_eur) {
                    return Err("Ungültige Kredithistorie.".into());
                }
                previous = e.time;
            }
        }
        if principal != i128::from(f.debt) {
            return Err("Kredithistorie und Restschuld widersprechen sich.".into());
        }
        Ok(())
    }
}
