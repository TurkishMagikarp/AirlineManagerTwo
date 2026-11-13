//! Staff v1: pilots, cabin crew and technicians, identical for players and AI.
//!
//! Pooled and deterministic, no individual qualifications or shifts:
//! - A flight plan *reserves* the pilots and cabin crew of its aircraft model for
//!   as long as it (or its post-maintenance resumption) exists. A plan cannot be
//!   created without enough unreserved crew, so no flight departs understaffed.
//!   Reserved crew cannot be dismissed.
//! - A hangar visit needs the model's technicians. Without enough free technicians
//!   the aircraft is grounded and waits; the check only starts (and its duration
//!   only begins) once they are available.
//! - Salaries accrue per employee and are paid at fixed payroll boundaries, plus a
//!   pro-rated final payment on dismissal. Payments are debited and archived in the
//!   history like maintenance, so the existing balance validation stays exact.
//!
//! All values: `config/staff.json`.
use crate::{
    aircraft::{self, AircraftStatus},
    game::Game,
    history::Purchase,
    maintenance,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::LazyLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Pilot,
    CabinCrew,
    Technician,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Pilot, Role::CabinCrew, Role::Technician];

    pub fn key(self) -> &'static str {
        match self {
            Role::Pilot => "pilot",
            Role::CabinCrew => "cabin_crew",
            Role::Technician => "technician",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Role::Pilot => "Pilot",
            Role::CabinCrew => "Flugbegleiter",
            Role::Technician => "Techniker",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Role::Pilot => "Piloten",
            Role::CabinCrew => "Flugbegleiter",
            Role::Technician => "Techniker",
        }
    }

    pub fn parse(key: &str) -> Result<Self, String> {
        Role::ALL
            .into_iter()
            .find(|r| r.key() == key)
            .ok_or_else(|| "Unbekannte Rolle.".to_string())
    }
}

#[derive(Deserialize)]
struct Salaries {
    pilot: i64,
    cabin_crew: i64,
    technician: i64,
}

#[derive(Deserialize)]
pub struct Needs {
    pub pilots: u32,
    pub cabin_crew: u32,
    pub technicians: u32,
}

#[derive(Deserialize)]
struct Config {
    payroll_period_days: u64,
    salary_reserve_periods: u32,
    max_employees: usize,
    max_hire_batch: u32,
    salary_eur_per_period: Salaries,
    models: std::collections::HashMap<String, Needs>,
    first_names: Vec<String>,
    last_names: Vec<String>,
}

static CONFIG: LazyLock<Result<Config, String>> = LazyLock::new(|| {
    let c: Config = serde_json::from_str(include_str!("../../config/staff.json"))
        .map_err(|_| "Personalkonfiguration ist ungültig.".to_string())?;
    let invalid = || "Personalkonfiguration enthält ungültige Werte.".to_string();
    let s = &c.salary_eur_per_period;
    if !(1..=365).contains(&c.payroll_period_days)
        || c.salary_reserve_periods > 12
        || !(1..=100_000).contains(&c.max_employees)
        || !(1..=1000).contains(&c.max_hire_batch)
        || [s.pilot, s.cabin_crew, s.technician]
            .iter()
            .any(|v| !(1..=1_000_000).contains(v))
        || c.first_names.is_empty()
        || c.last_names.is_empty()
    {
        return Err(invalid());
    }
    for m in aircraft::models()? {
        let n = c
            .models
            .get(&m.id)
            .ok_or_else(|| format!("Personalbedarf fehlt für {}.", m.id))?;
        if [n.pilots, n.cabin_crew, n.technicians]
            .iter()
            .any(|v| !(1..=1000).contains(v))
        {
            return Err(invalid());
        }
    }
    Ok(c)
});

fn config() -> Result<&'static Config, String> {
    CONFIG.as_ref().map_err(Clone::clone)
}

pub fn needs(model_id: &str) -> Result<&'static Needs, String> {
    config()?
        .models
        .get(model_id)
        .ok_or_else(|| "Unbekanntes Flugzeugmodell.".to_string())
}

pub fn salary(role: Role) -> Result<i64, String> {
    let s = &config()?.salary_eur_per_period;
    Ok(match role {
        Role::Pilot => s.pilot,
        Role::CabinCrew => s.cabin_crew,
        Role::Technician => s.technician,
    })
}

pub fn period_seconds() -> Result<u64, String> {
    Ok(config()?.payroll_period_days * 86_400)
}

pub fn max_hire_batch() -> u32 {
    config().map_or(1, |c| c.max_hire_batch)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Employee {
    pub id: u32,
    pub role: Role,
    pub name: String,
    /// Salary per payroll period (a 30-day "month" by default).
    pub salary_eur: i64,
    pub hired_at: u64,
    /// Salary is settled up to this instant.
    pub paid_until: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Staff {
    pub employees: Vec<Employee>,
    /// 0 in older saves; ids start at 1.
    #[serde(default)]
    pub next_id: u32,
}

/// What the employee is doing right now, derived from fleet state: the
/// lowest ids are the ones assigned, so dismissal of a free employee is stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Available,
    /// Pilot or cabin crew reserved by a flight plan.
    Flying,
    /// Technician working on an aircraft in the hangar.
    Hangar,
}

impl Activity {
    pub fn label(self) -> &'static str {
        match self {
            Activity::Available => "Verfügbar",
            Activity::Flying => "Im Flugdienst",
            Activity::Hangar => "In der Wartung",
        }
    }
}

impl Game {
    pub fn staff_count(&self, role: Role) -> u32 {
        self.staff
            .employees
            .iter()
            .filter(|e| e.role == role)
            .count() as u32
    }

    /// Pilots and cabin crew held by flight plans (including plans suspended
    /// for a check, which resume automatically).
    pub fn reserved_crew(&self) -> (u32, u32) {
        let (mut pilots, mut cabin) = (0, 0);
        for plane in &self.fleet {
            let held = self.planned_route(plane.id).is_some()
                || plane
                    .maintenance
                    .as_ref()
                    .is_some_and(|v| v.resume.is_some());
            if let (true, Ok(n)) = (held, needs(&plane.model_id)) {
                pilots += n.pilots;
                cabin += n.cabin_crew;
            }
        }
        (pilots, cabin)
    }

    pub fn busy_technicians(&self) -> u32 {
        self.fleet
            .iter()
            .filter(|p| {
                p.status == AircraftStatus::Maintenance
                    && p.maintenance.as_ref().is_some_and(|v| v.staffed)
            })
            .filter_map(|p| needs(&p.model_id).ok())
            .map(|n| n.technicians)
            .sum()
    }

    pub fn busy_staff(&self, role: Role) -> u32 {
        match role {
            Role::Pilot => self.reserved_crew().0,
            Role::CabinCrew => self.reserved_crew().1,
            Role::Technician => self.busy_technicians(),
        }
    }

    pub fn available_staff(&self, role: Role) -> u32 {
        self.staff_count(role).saturating_sub(self.busy_staff(role))
    }

    /// Technicians needed to service the largest aircraft of the fleet.
    pub fn technician_target(&self) -> u32 {
        self.fleet
            .iter()
            .filter_map(|p| needs(&p.model_id).ok())
            .map(|n| n.technicians)
            .max()
            .unwrap_or(0)
    }

    /// Unreserved pilots / cabin crew missing to fly one more aircraft of the model.
    pub fn crew_shortage(&self, model_id: &str) -> Result<(u32, u32), String> {
        let n = needs(model_id)?;
        Ok((
            n.pilots.saturating_sub(self.available_staff(Role::Pilot)),
            n.cabin_crew
                .saturating_sub(self.available_staff(Role::CabinCrew)),
        ))
    }

    pub fn require_crew(&self, model_id: &str) -> Result<(), String> {
        let (pilots, cabin) = self.crew_shortage(model_id)?;
        if pilots == 0 && cabin == 0 {
            return Ok(());
        }
        let mut parts = Vec::new();
        if pilots > 0 {
            parts.push(format!("{pilots} Piloten"));
        }
        if cabin > 0 {
            parts.push(format!("{cabin} Flugbegleiter"));
        }
        Err(format!(
            "Besatzung fehlt: {} · bitte im Personal-Tab einstellen. Ohne Besatzung startet kein Flug.",
            parts.join(" und ")
        ))
    }

    /// Per-employee activity, in employee order.
    pub fn staff_activity(&self) -> Vec<Activity> {
        let mut used = [0u32; 3];
        let busy = [
            self.busy_staff(Role::Pilot),
            self.busy_staff(Role::CabinCrew),
            self.busy_staff(Role::Technician),
        ];
        self.staff
            .employees
            .iter()
            .map(|e| {
                let i = Role::ALL.iter().position(|r| *r == e.role).unwrap();
                if used[i] < busy[i] {
                    used[i] += 1;
                    if e.role == Role::Technician {
                        Activity::Hangar
                    } else {
                        Activity::Flying
                    }
                } else {
                    Activity::Available
                }
            })
            .collect()
    }

    pub fn monthly_payroll(&self) -> i64 {
        self.staff
            .employees
            .iter()
            .fold(0i64, |sum, e| sum.saturating_add(e.salary_eur))
    }

    fn employee_name(id: u32) -> Result<String, String> {
        let c = config()?;
        let i = id as usize;
        Ok(format!(
            "{} {}",
            c.first_names[(i * 7 + 3) % c.first_names.len()],
            c.last_names[(i * 13 + 5) % c.last_names.len()]
        ))
    }

    /// Atomic: nothing changes unless every role can be hired. `check_funds`
    /// requires accrued pay plus the configured reserve for the entire resulting
    /// workforce in cash (not for save migration). Nothing is charged upfront.
    pub(crate) fn hire_batch(
        &mut self,
        hires: &[(Role, u32)],
        check_funds: bool,
    ) -> Result<(), String> {
        let c = config()?;
        let now = self.clock.elapsed().as_secs();
        let airline = self
            .airline
            .as_ref()
            .ok_or("Bitte zuerst eine Airline gründen.")?;
        let total_count = hires.iter().try_fold(0u32, |sum, h| {
            sum.checked_add(h.1).ok_or("Ungültige Anzahl.")
        })?;
        if total_count == 0 {
            return Ok(());
        }
        if total_count > c.max_hire_batch && check_funds {
            return Err(format!(
                "Höchstens {} Mitarbeiter pro Einstellung.",
                c.max_hire_batch
            ));
        }
        if self.staff.employees.len() + total_count as usize > c.max_employees {
            return Err(format!(
                "Höchstens {} Mitarbeiter möglich.",
                c.max_employees
            ));
        }
        let mut cost = 0i64;
        for (role, count) in hires {
            cost = cost
                .checked_add(
                    salary(*role)?
                        .checked_mul(i64::from(*count))
                        .ok_or("Finanzüberlauf.")?,
                )
                .ok_or("Finanzüberlauf.")?;
        }
        let required = self
            .monthly_payroll()
            .checked_add(cost)
            .and_then(|v| v.checked_mul(i64::from(c.salary_reserve_periods)))
            .and_then(|v| v.checked_add(self.accrued_payroll().ok()?))
            .ok_or("Finanzüberlauf.")?;
        if check_funds && airline.capital_eur < required {
            return Err(format!(
                "Nicht genügend Guthaben: {required} EUR für offene Gehälter und Personalreserve benötigt, {} EUR verfügbar.",
                airline.capital_eur
            ));
        }
        let first_id = self.staff.next_id.max(1);
        first_id
            .checked_add(total_count)
            .ok_or("Keine weitere Mitarbeiterkennung verfügbar.")?;
        let mut id = first_id;
        let mut staff = self.staff.clone();
        for (role, count) in hires {
            for _ in 0..*count {
                staff.employees.push(Employee {
                    id,
                    role: *role,
                    name: Self::employee_name(id)?,
                    salary_eur: salary(*role)?,
                    hired_at: now,
                    paid_until: now,
                });
                id += 1;
            }
        }
        staff.next_id = id;
        let previous_staff = std::mem::replace(&mut self.staff, staff);
        let previous_fleet = self.fleet.clone();
        if let Err(error) = self.assign_waiting_technicians() {
            self.staff = previous_staff;
            self.fleet = previous_fleet;
            return Err(error);
        }
        Ok(())
    }

    pub fn hire_staff(&mut self, role: Role, count: u32) -> Result<(), String> {
        if count == 0 {
            return Err("Bitte mindestens einen Mitarbeiter einstellen.".into());
        }
        self.hire_batch(&[(role, count)], true)
    }

    /// Hire exactly what is missing so one more aircraft of `model_id` can fly
    /// and the hangar can service the fleet. Used by AI airlines and migration.
    pub fn equip_staff_for(&mut self, model_id: &str) -> Result<(), String> {
        let n = needs(model_id)?;
        let (pilots, cabin) = self.crew_shortage(model_id)?;
        let techs = self
            .technician_target()
            .max(n.technicians)
            .saturating_sub(self.staff_count(Role::Technician));
        self.hire_batch(
            &[
                (Role::Pilot, pilots),
                (Role::CabinCrew, cabin),
                (Role::Technician, techs),
            ],
            true,
        )
    }

    /// Salary owed to `e` up to `until` since the last settlement.
    fn accrued(e: &Employee, until: u64) -> Result<i64, String> {
        let period = i128::from(period_seconds()?);
        // Cumulative rounding carries fractional EUR across payroll boundaries.
        let earned = i128::from(e.salary_eur) * i128::from(until.saturating_sub(e.hired_at));
        let paid = i128::from(e.salary_eur) * i128::from(e.paid_until.saturating_sub(e.hired_at));
        i64::try_from(earned / period - paid / period).map_err(|_| "Finanzüberlauf.".to_string())
    }

    pub fn accrued_payroll(&self) -> Result<i64, String> {
        self.staff.employees.iter().try_fold(0i64, |sum, e| {
            sum.checked_add(Self::accrued(e, self.clock.elapsed().as_secs())?)
                .ok_or_else(|| "Finanzüberlauf.".to_string())
        })
    }

    fn record_payroll(&mut self, time: u64, amount: i64) {
        if amount <= 0 {
            return;
        }
        if let Some(h) = &mut self.history {
            let time = h.payroll.last().map_or(time, |p| p.time.max(time));
            h.payroll.push(Purchase { time, amount });
        }
    }

    /// Dismiss one employee immediately; the salary accrued so far is paid.
    pub fn dismiss_staff(&mut self, id: u32) -> Result<(), String> {
        if self.airline.is_none() {
            return Err("Bitte zuerst eine Airline gründen.".into());
        }
        let i = self
            .staff
            .employees
            .iter()
            .position(|e| e.id == id)
            .ok_or("Unbekannter Mitarbeiter.")?;
        if self.staff_activity()[i] != Activity::Available {
            let e = &self.staff.employees[i];
            return Err(format!(
                "{} ({}) ist im Einsatz und kann nicht entlassen werden. Zuerst Flugplan beenden bzw. Wartung abwarten.",
                e.name,
                e.role.label()
            ));
        }
        self.remove_employee(i)
    }

    fn remove_employee(&mut self, i: usize) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        let e = &self.staff.employees[i];
        let mut pay = Self::accrued(e, now)?;
        // Final settlement rounds up, preventing repeated short employment from
        // providing free labor. Existing archived payments remain untouched.
        let earned = i128::from(e.salary_eur) * i128::from(now.saturating_sub(e.hired_at));
        if earned % i128::from(period_seconds()?) != 0 {
            pay = pay.checked_add(1).ok_or("Finanzüberlauf.")?;
        }
        let balance = self
            .airline
            .as_ref()
            .ok_or("Airline fehlt.")?
            .capital_eur
            .checked_sub(pay)
            .ok_or("Finanzüberlauf.")?;
        self.airline.as_mut().unwrap().capital_eur = balance;
        self.record_payroll(now, pay);
        self.staff.employees.remove(i);
        Ok(())
    }

    /// AI housekeeping: let go of free staff beyond the current needs.
    pub fn dismiss_surplus(&mut self) -> Result<(), String> {
        for role in Role::ALL {
            let keep = match role {
                Role::Technician => self.technician_target().max(self.busy_technicians()),
                _ => self.busy_staff(role),
            };
            while self.staff_count(role) > keep {
                let i = self
                    .staff
                    .employees
                    .iter()
                    .rposition(|e| e.role == role)
                    .unwrap();
                self.remove_employee(i)?;
            }
        }
        Ok(())
    }

    /// Hangar visits waiting for technicians start as soon as enough are free.
    /// Deterministic: fleet order, evaluated at hires, dismissals and events.
    pub(crate) fn assign_waiting_technicians(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        for p in 0..self.fleet.len() {
            let plane = &self.fleet[p];
            let Some(v) = &plane.maintenance else {
                continue;
            };
            if plane.status != AircraftStatus::Maintenance || v.staffed {
                continue;
            }
            if self.available_staff(Role::Technician) < needs(&plane.model_id)?.technicians {
                continue;
            }
            let end = now
                .checked_add(maintenance::duration(
                    maintenance::rules(&plane.model_id)?,
                    v.unscheduled,
                )?)
                .ok_or("Wartungs-Zeitüberlauf.")?;
            let v = self.fleet[p].maintenance.as_mut().unwrap();
            v.staffed = true;
            v.end = end;
        }
        Ok(())
    }

    /// Next whole-second payroll boundary after now, if anyone is employed.
    pub fn next_payroll_event(&self) -> Option<u64> {
        if self.staff.employees.is_empty() {
            return None;
        }
        let period = period_seconds().ok()?;
        (self.clock.elapsed().as_secs() / period)
            .checked_add(1)?
            .checked_mul(period)
    }

    /// Settle all salaries up to the latest payroll boundary at or before now.
    pub(crate) fn apply_payroll(&mut self) -> Result<(), String> {
        if self.airline.is_none() || self.staff.employees.is_empty() {
            return Ok(());
        }
        let period = period_seconds()?;
        let due = self.clock.elapsed().as_secs() / period * period;
        let mut total = 0i64;
        for e in self.staff.employees.iter().filter(|e| e.paid_until < due) {
            total = total
                .checked_add(Self::accrued(e, due)?)
                .ok_or("Finanzüberlauf.")?;
        }
        let balance = self
            .airline
            .as_ref()
            .unwrap()
            .capital_eur
            .checked_sub(total)
            .ok_or("Finanzüberlauf.")?;
        for e in &mut self.staff.employees {
            e.paid_until = e.paid_until.max(due);
        }
        self.airline.as_mut().unwrap().capital_eur = balance;
        self.record_payroll(due, total);
        Ok(())
    }

    /// Old saves had no staff: employ the minimum for flight plans that already
    /// exist and a hangar crew, so nothing stops on migration.
    pub(crate) fn grandfather_staff(&mut self) -> Result<(), String> {
        if self.airline.is_none() || !self.staff.employees.is_empty() {
            return Ok(());
        }
        let (pilots, cabin) = self.reserved_crew();
        let techs = self.technician_target().max(self.busy_technicians());
        self.hire_batch(
            &[
                (Role::Pilot, pilots),
                (Role::CabinCrew, cabin),
                (Role::Technician, techs),
            ],
            false,
        )
    }

    pub fn validate_staff(&self) -> Result<(), String> {
        let invalid = || "Ungültige Personaldaten im Spielstand.".to_string();
        let now = self.clock.elapsed().as_secs();
        if self.airline.is_none() && !self.staff.employees.is_empty() {
            return Err(invalid());
        }
        if self.staff.employees.len() > config()?.max_employees {
            return Err(invalid());
        }
        let mut ids = HashSet::new();
        for e in &self.staff.employees {
            if e.id == 0
                || e.id >= self.staff.next_id
                || !ids.insert(e.id)
                || !(1..=1_000_000).contains(&e.salary_eur)
                || e.name.is_empty()
                || e.name.chars().count() > 80
                || e.hired_at > now
                || e.paid_until < e.hired_at
                || e.paid_until > now
            {
                return Err(invalid());
            }
        }
        if self.staff.employees.windows(2).any(|w| w[0].id >= w[1].id) {
            return Err(invalid());
        }
        if let Some(h) = &self.history
            && (h
                .payroll
                .iter()
                .any(|p| p.time < h.since || p.time > now || p.amount <= 0)
                || h.payroll.windows(2).any(|p| p[0].time > p[1].time))
        {
            return Err(invalid());
        }
        Ok(())
    }
}

#[cfg(test)]
impl Game {
    /// Test fixture: employ enough staff to fly every aircraft at once.
    pub fn staff_up(&mut self) {
        let (mut pilots, mut cabin) = (0, 0);
        for plane in &self.fleet {
            let n = needs(&plane.model_id).unwrap();
            pilots += n.pilots;
            cabin += n.cabin_crew;
        }
        let missing = |have: u32, need: u32| need.saturating_sub(have);
        let techs = missing(self.staff_count(Role::Technician), self.technician_target());
        self.hire_batch(
            &[
                (Role::Pilot, missing(self.staff_count(Role::Pilot), pilots)),
                (
                    Role::CabinCrew,
                    missing(self.staff_count(Role::CabinCrew), cabin),
                ),
                (Role::Technician, techs),
            ],
            false,
        )
        .unwrap();
    }
}
