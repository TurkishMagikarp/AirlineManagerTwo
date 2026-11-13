//! Annual home-country income tax. All amounts are whole EUR.
use crate::{demand::YEAR_SECONDS, game::Game, history::History};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Deserialize)]
struct Rules {
    default_rate_bps: u32,
    country_rates_bps: BTreeMap<String, u32>,
}
static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/tax.json"))
        .expect("valid tax configuration");
    assert!(r.default_rate_bps <= 10_000);
    assert!(r.country_rates_bps.values().all(|r| *r <= 10_000));
    r
});
pub fn rate(country: &str) -> u32 {
    RULES
        .country_rates_bps
        .get(country)
        .copied()
        .unwrap_or(RULES.default_rate_bps)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Tax {
    /// Load point for migrated saves; founding instant for new companies.
    pub since: u64,
    pub opening_profit: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Assessment {
    pub time: u64,
    pub profit: i64,
    /// Frozen cumulative pre-tax result; later actions at this timestamp belong
    /// to the new year and must not rewrite the closed year.
    pub closing_profit: i64,
    pub rate_bps: u32,
    pub amount: i64,
    pub loss: i64,
}
fn assess(
    time: u64,
    profit: i64,
    loss: i64,
    rate_bps: u32,
    closing_profit: i64,
) -> Result<Assessment, String> {
    let net = i128::from(profit) - i128::from(loss);
    let amount = i64::try_from((net.max(0) * i128::from(rate_bps) + 5000) / 10_000)
        .map_err(|_| "Steuerüberlauf.")?;
    let loss = i64::try_from((-net).max(0)).map_err(|_| "Verlustüberlauf.")?;
    Ok(Assessment {
        time,
        profit,
        closing_profit,
        rate_bps,
        amount,
        loss,
    })
}
impl History {
    pub fn tax_through(&self, time: u64) -> i128 {
        self.taxes
            .iter()
            .filter(|a| a.time <= time)
            .map(|a| i128::from(a.amount))
            .sum()
    }
    pub fn pretax_profit(&self, time: u64) -> Result<i64, String> {
        i64::try_from(
            i128::from(self.prior_totals.profit()) + i128::from(self.totals(0, time).profit())
                - self.maintenance_through(time)
                - self.payroll_through(time)
                - self.interest_through(time)
                - self.rent_through(time)
                - self.marketing_through(time),
        )
        .map_err(|_| "Gewinnüberlauf.".into())
    }
}
impl Game {
    pub fn tax_rate(&self) -> u32 {
        self.airline
            .as_ref()
            .and_then(|a| {
                crate::airports::catalog()
                    .ok()?
                    .airports
                    .iter()
                    .find(|p| p.icao == a.home_airport_icao)
            })
            .map_or(RULES.default_rate_bps, |p| rate(&p.country_code))
    }
    pub fn tax_loss(&self) -> i64 {
        self.history
            .as_ref()
            .and_then(|h| h.taxes.last())
            .map_or(0, |a| a.loss)
    }
    pub fn next_tax_event(&self) -> Option<u64> {
        self.airline.as_ref()?;
        if self.insolvent() {
            return None;
        }
        let last = self
            .history
            .as_ref()?
            .taxes
            .last()
            .map_or(self.tax.since, |a| a.time);
        (last / YEAR_SECONDS)
            .checked_add(1)?
            .checked_mul(YEAR_SECONDS)
    }
    pub fn apply_tax(&mut self) -> Result<(), String> {
        let Some(due) = self.next_tax_event() else {
            return Ok(());
        };
        if due > self.clock.elapsed().as_secs() {
            return Ok(());
        }
        let h = self.history.as_ref().ok_or("Finanzhistorie fehlt.")?;
        let baseline = match h.taxes.last() {
            Some(a) => a.closing_profit,
            None => self.tax.opening_profit,
        };
        let closing_profit = h.pretax_profit(due)?;
        let profit = closing_profit
            .checked_sub(baseline)
            .ok_or("Gewinnüberlauf.")?;
        let a = assess(
            due,
            profit,
            self.tax_loss(),
            self.tax_rate(),
            closing_profit,
        )?;
        let airline = self.airline.as_mut().ok_or("Airline fehlt.")?;
        airline.capital_eur = airline
            .capital_eur
            .checked_sub(a.amount)
            .ok_or("Steuerüberlauf.")?;
        self.history.as_mut().unwrap().taxes.push(a);
        self.observe_distress();
        Ok(())
    }
    pub fn migrate_tax(&mut self) -> Result<(), String> {
        if self.airline.is_none() {
            self.tax = Tax::default();
            return Ok(());
        }
        self.tax = Tax {
            since: self.clock.elapsed().as_secs(),
            opening_profit: self
                .history
                .as_ref()
                .map_or(Ok(0), |h| h.pretax_profit(u64::MAX))?,
        };
        if let Some(h) = &mut self.history {
            h.taxes.clear();
        }
        Ok(())
    }
    pub fn validate_tax(&self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        if self.tax.since > now {
            return Err("Ungültiger Steuerbeginn.".into());
        }
        let Some(h) = &self.history else {
            return if self.tax == Tax::default() {
                Ok(())
            } else {
                Err("Steuerdaten ohne Airline.".into())
            };
        };
        if self.tax.since < h.since {
            return Err("Ungültige Steuerbasis.".into());
        }
        let mut last = self.tax.since;
        let mut baseline = self.tax.opening_profit;
        let mut loss = 0;
        for a in &h.taxes {
            let due = (last / YEAR_SECONDS)
                .checked_add(1)
                .and_then(|y| y.checked_mul(YEAR_SECONDS))
                .ok_or("Zeitüberlauf.")?;
            let total = a.closing_profit;
            let profit = total.checked_sub(baseline).ok_or("Gewinnüberlauf.")?;
            if a.time != due
                || a.time > now
                || a.rate_bps > 10_000
                || *a != assess(due, profit, loss, a.rate_bps, total)?
            {
                return Err("Ungültiges Steuerarchiv.".into());
            }
            last = a.time;
            baseline = total;
            loss = a.loss;
        }
        if self.next_tax_event().is_some_and(|t| t <= now) {
            return Err("Steuerabschluss fehlt.".into());
        }
        Ok(())
    }
}
