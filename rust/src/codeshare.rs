//! Bilateral player–AI agreements. Receipts are frozen with each departure.
use crate::{demand::Offer, economy::Totals, game::Game};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Rules {
    pub maximum_partners: usize,
    pub minimum_cash_eur: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/codeshare.json"))
        .expect("codeshare config");
    assert!((1..=4).contains(&r.maximum_partners));
    assert!(r.minimum_cash_eur >= 0);
    r
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agreement {
    pub partner: String,
    pub started_at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub partner: String,
    pub passengers: i64,
    pub revenue: i64,
}
pub fn add_receipt(receipts: &mut Vec<Receipt>, other: &Receipt, count: u64) -> Option<()> {
    if count == 0 {
        return Some(());
    }
    let count = i64::try_from(count).ok()?;
    let passengers = other.passengers.checked_mul(count)?;
    let revenue = other.revenue.checked_mul(count)?;
    match receipts.iter_mut().find(|r| r.partner == other.partner) {
        Some(r) => {
            r.passengers = r.passengers.checked_add(passengers)?;
            r.revenue = r.revenue.checked_add(revenue)?;
        }
        None => {
            receipts.push(Receipt {
                partner: other.partner.clone(),
                passengers,
                revenue,
            });
            receipts.sort_by(|a, b| a.partner.cmp(&b.partner));
        }
    }
    Some(())
}
pub fn valid_receipts(t: &Totals) -> bool {
    let mut passengers = 0i64;
    let mut revenue = 0i64;
    let mut previous: Option<&str> = None;
    for r in &t.codeshare {
        if r.partner.is_empty()
            || r.passengers <= 0
            || r.revenue < 0
            || previous.is_some_and(|p| p >= r.partner.as_str())
        {
            return false;
        }
        let Some(p) = passengers.checked_add(r.passengers) else {
            return false;
        };
        let Some(v) = revenue.checked_add(r.revenue) else {
            return false;
        };
        passengers = p;
        revenue = v;
        previous = Some(&r.partner);
    }
    passengers <= t.connecting_passengers && revenue <= t.connecting_revenue
}
fn unavailable(g: &Game) -> bool {
    g.airline.is_none() || g.insolvent() || g.resolution.deadline.is_some()
}
impl Game {
    pub fn codeshare_reason(&self, partner: &str) -> Result<(), String> {
        self.require_active()?;
        let c = self
            .competitors
            .iter()
            .find(|c| c.game.market_owner_key() == partner)
            .ok_or("CODESHARE_MISSING")?;
        if self.codeshares.iter().any(|a| a.partner == partner) {
            return Err("CODESHARE_EXISTS".into());
        }
        if unavailable(self)
            || unavailable(&c.game)
            || self.distressed()
            || c.game.distressed()
            || self.cash() < RULES.minimum_cash_eur
            || c.game.cash() < RULES.minimum_cash_eur
        {
            return Err("CODESHARE_FINANCES".into());
        }
        if let Some(mine) = self.alliance_of(crate::alliances::PLAYER)
            && self
                .alliance_of(partner)
                .is_some_and(|m| m.alliance != mine.alliance)
        {
            return Err("CODESHARE_RIVAL_ALLIANCE".into());
        }
        if self.codeshares.len() >= RULES.maximum_partners {
            return Err("CODESHARE_LIMIT".into());
        }
        let active_airports = |g: &Game| -> std::collections::BTreeSet<String> {
            g.routes
                .iter()
                .filter(|r| r.plan.is_some())
                .flat_map(|r| [r.origin_icao.clone(), r.destination_icao.clone()])
                .collect()
        };
        if active_airports(self).is_disjoint(&active_airports(&c.game)) {
            return Err("CODESHARE_NO_HUB".into());
        }
        Ok(())
    }
    pub fn propose_codeshare(&mut self, partner: &str) -> Result<(), String> {
        self.codeshare_reason(partner)?;
        self.codeshares.push(Agreement {
            partner: partner.into(),
            started_at: self.clock.elapsed().as_secs(),
        });
        self.codeshares.sort_by(|a, b| a.partner.cmp(&b.partner));
        self.commercial_changed();
        Ok(())
    }
    pub fn end_codeshare(&mut self, partner: &str) -> Result<(), String> {
        if !self.codeshares.iter().any(|a| a.partner == partner) {
            return Err("CODESHARE_MISSING".into());
        }
        self.codeshares.retain(|a| a.partner != partner);
        self.commercial_changed();
        Ok(())
    }
    pub(crate) fn review_codeshares(&mut self) {
        let before = self.codeshares.len();
        let player_unavailable = unavailable(self);
        self.codeshares.retain(|a| {
            !player_unavailable
                && self
                    .competitors
                    .iter()
                    .any(|c| c.game.market_owner_key() == a.partner && !unavailable(&c.game))
        });
        if before != self.codeshares.len() {
            self.commercial_changed();
        }
    }
    pub(crate) fn apply_codeshares(&self, offers: &mut [Offer]) {
        if self.codeshares.is_empty() {
            return;
        }
        let player = self.market_owner_key();
        for a in &self.codeshares {
            if unavailable(self)
                || !self
                    .competitors
                    .iter()
                    .any(|c| c.game.market_owner_key() == a.partner && !unavailable(&c.game))
            {
                continue;
            }
            for o in &mut *offers {
                if o.owner == player {
                    o.codeshare_partners.push(a.partner.clone());
                } else if o.owner == a.partner {
                    o.codeshare_partners.push(player.clone());
                }
            }
        }
    }
    pub fn validate_codeshares(&self) -> Result<(), String> {
        if self.codeshares.len() > RULES.maximum_partners {
            return Err("Ungültige Codeshares.".into());
        }
        let mut seen = std::collections::HashSet::new();
        for a in &self.codeshares {
            if a.started_at > self.clock.elapsed().as_secs()
                || !seen.insert(&a.partner)
                || !self
                    .competitors
                    .iter()
                    .any(|c| c.game.market_owner_key() == a.partner)
                || self
                    .competitors
                    .iter()
                    .any(|c| !c.game.codeshares.is_empty())
            {
                return Err("Ungültige Codeshares.".into());
            }
        }
        Ok(())
    }
}
