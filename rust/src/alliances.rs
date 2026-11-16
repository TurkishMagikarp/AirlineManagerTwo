//! Airline alliances: AI airlines form and join them, the player may apply. Members
//! share connecting passengers through the codeshare offer mechanism (see `codeshare`),
//! pay a monthly fee through their own ledger and get a small demand bonus.
//! Every decision happens on a fixed review cycle, never on frame or step boundaries.
use crate::{demand::Offer, game::Game, history::Purchase};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

const DAY: u64 = 86400;
const NOTICE_LIMIT: usize = 8;
/// Owner marker of the player's membership; AI members use their market owner key.
pub const PLAYER: &str = "@player";

#[derive(Deserialize)]
pub struct Def {
    pub id: String,
    pub name: String,
}
#[derive(Deserialize)]
pub struct Rules {
    pub review_days: u64,
    pub leave_notice_days: u64,
    pub reapply_cooldown_days: u64,
    pub notice_visible_days: u64,
    pub max_members: usize,
    pub min_reputation_points: i64,
    pub min_routes: usize,
    pub max_overlap_percent: u32,
    pub ai_max_overlap_percent: u32,
    pub ai_leave_below_routes: usize,
    pub min_cash_months: i64,
    pub fee_base_eur: i64,
    pub fee_per_route_eur: i64,
    pub demand_bonus_bps_per_partner: i64,
    pub demand_bonus_max_bps: i64,
    pub alliances: Vec<Def>,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/alliances.json"))
        .expect("alliances config");
    assert!((1..=365).contains(&r.review_days));
    assert!(r.leave_notice_days <= 365 && r.reapply_cooldown_days <= 365);
    assert!((2..=8).contains(&r.max_members));
    assert!((-100..=100).contains(&r.min_reputation_points));
    assert!(r.min_routes >= 1 && r.ai_leave_below_routes <= r.min_routes);
    assert!(r.max_overlap_percent <= 100 && r.ai_max_overlap_percent <= 100);
    assert!((0..=12).contains(&r.min_cash_months));
    assert!(r.fee_base_eur >= 0 && r.fee_per_route_eur >= 0);
    assert!(r.fee_base_eur + r.fee_per_route_eur > 0);
    assert!(
        (0..=1000).contains(&r.demand_bonus_bps_per_partner)
            && (0..=2000).contains(&r.demand_bonus_max_bps)
    );
    assert!((1..=8).contains(&r.alliances.len()));
    let ids: BTreeSet<_> = r.alliances.iter().map(|a| &a.id).collect();
    assert_eq!(ids.len(), r.alliances.len());
    r
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub owner: String,
    pub alliance: String,
    pub joined_at: u64,
    /// Notice period end; the membership stops at the first review at or after it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaving_at: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Application {
    pub alliance: String,
    pub applied_at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub id: u64,
    pub at: u64,
    pub kind: String,
    pub alliance: String,
    pub text: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// Time of the last review (or of loading an older save); the cycle is derived from it.
    pub reviewed_at: u64,
    pub members: Vec<Member>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<Application>,
    #[serde(default)]
    pub blocked_until: u64,
    #[serde(default)]
    pub notices: Vec<Notice>,
    #[serde(default)]
    pub next_notice: u64,
}
impl State {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A plain, explainable refusal. `code` is a localization key, `args` fill its placeholders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: &'static str,
    pub args: Vec<String>,
}
fn refuse(code: &'static str, args: &[String]) -> Refusal {
    Refusal {
        code,
        args: args.to_vec(),
    }
}
impl Refusal {
    pub fn text(&self) -> String {
        let a = |i: usize| self.args.get(i).map_or("", String::as_str);
        match self.code {
            "ALLIANCE_RESTRUCTURING" => "Finanznot oder Restrukturierung".into(),
            "ALLIANCE_COOLDOWN" => format!("Wiederbewerbung erst ab {}", a(0)),
            "ALLIANCE_ACTIVE" => "Mitgliedschaft oder Antrag bereits aktiv".into(),
            "ALLIANCE_EMPTY" => "Allianz hat noch keine Mitglieder".into(),
            "ALLIANCE_FULL" => format!("Allianz voll ({} Mitglieder)", a(0)),
            "ALLIANCE_REPUTATION" => format!("Pünktlichkeitsruf {} < Mindestwert {}", a(0), a(1)),
            "ALLIANCE_SIZE" => format!("Nur {} aktive Strecken, mindestens {}", a(0), a(1)),
            "ALLIANCE_CASH" => format!("Guthaben {} EUR unter Reserve von {} EUR", a(0), a(1)),
            "ALLIANCE_OVERLAP" => format!("Netzüberschneidung {} % > erlaubte {} %", a(0), a(1)),
            _ => self.code.into(),
        }
    }
}

pub fn def(id: &str) -> Option<&'static Def> {
    RULES.alliances.iter().find(|a| a.id == id)
}
fn cycle() -> u64 {
    RULES.review_days * DAY
}
fn unavailable(g: &Game) -> bool {
    g.airline.is_none() || g.insolvent() || g.resolution.deadline.is_some()
}
fn active_routes(g: &Game) -> usize {
    g.routes
        .iter()
        .filter(|r| r.plan.is_some())
        .map(|r| r.public_id())
        .collect::<BTreeSet<_>>()
        .len()
}
fn airports(g: &Game) -> BTreeSet<String> {
    g.routes
        .iter()
        .filter(|r| r.plan.is_some())
        .flat_map(|r| [r.origin_icao.clone(), r.destination_icao.clone()])
        .collect()
}
pub fn monthly_fee(g: &Game) -> i64 {
    RULES.fee_base_eur + RULES.fee_per_route_eur * active_routes(g) as i64
}
fn charge(g: &mut Game, now: u64, amount: i64) -> Result<(), String> {
    let a = g.airline.as_mut().ok_or("Airline fehlt.")?;
    a.capital_eur = a
        .capital_eur
        .checked_sub(amount)
        .ok_or("Allianzgebührenüberlauf.")?;
    g.history
        .as_mut()
        .ok_or("Finanzhistorie fehlt.")?
        .alliance_fees
        .push(Purchase { time: now, amount });
    g.observe_distress();
    Ok(())
}
impl crate::history::History {
    pub fn alliance_fee_through(&self, time: u64) -> i128 {
        self.alliance_fees
            .iter()
            .filter(|p| p.time <= time)
            .map(|p| i128::from(p.amount))
            .sum()
    }
}

impl Game {
    fn alliance_game(&self, owner: &str) -> Option<&Game> {
        if owner == PLAYER {
            return Some(self);
        }
        self.competitors
            .iter()
            .map(|c| &*c.game)
            .find(|g| g.market_owner_key() == owner)
    }
    fn alliance_game_mut(&mut self, owner: &str) -> Option<&mut Game> {
        if owner == PLAYER {
            return Some(self);
        }
        self.competitors
            .iter_mut()
            .map(|c| &mut *c.game)
            .find(|g| g.market_owner_key() == owner)
    }
    /// The key under which an owner's offers appear in the market.
    fn alliance_offer_key(&self, owner: &str) -> String {
        if owner == PLAYER {
            self.market_owner_key()
        } else {
            owner.into()
        }
    }
    pub fn alliance_of(&self, owner: &str) -> Option<&Member> {
        self.alliances.members.iter().find(|m| m.owner == owner)
    }
    pub fn alliance_members(&self, alliance: &str) -> impl Iterator<Item = &Member> {
        let alliance = alliance.to_owned();
        self.alliances
            .members
            .iter()
            .filter(move |m| m.alliance == alliance)
    }
    /// Shared airports (count) and the largest share of the candidate's own airports
    /// that a single member already serves.
    pub fn alliance_overlap(&self, owner: &str, alliance: &str) -> (u32, usize) {
        let Some(me) = self.alliance_game(owner).map(airports) else {
            return (0, 0);
        };
        let mut all = BTreeSet::new();
        let mut worst = 0u32;
        for m in self.alliance_members(alliance).filter(|m| m.owner != owner) {
            let Some(other) = self.alliance_game(&m.owner).map(airports) else {
                continue;
            };
            let shared = me.intersection(&other).count();
            if !me.is_empty() {
                worst = worst.max((shared * 100 / me.len()) as u32);
            }
            all.extend(me.intersection(&other).cloned());
        }
        (worst, all.len())
    }
    /// Entry conditions shared by the player and AI airlines. An AI may found an empty alliance.
    pub fn alliance_check(&self, owner: &str, alliance: &str) -> Result<(), Refusal> {
        let now = self.clock.elapsed().as_secs();
        let ai = owner != PLAYER;
        let g = self
            .alliance_game(owner)
            .ok_or_else(|| refuse("ALLIANCE_RESTRUCTURING", &[]))?;
        if unavailable(g) || g.distressed() {
            return Err(refuse("ALLIANCE_RESTRUCTURING", &[]));
        }
        if !ai && self.alliances.blocked_until > now {
            return Err(refuse(
                "ALLIANCE_COOLDOWN",
                &[crate::operations::display_time(self.alliances.blocked_until)],
            ));
        }
        if self.alliance_of(owner).is_some()
            || (!ai && self.alliances.application.is_some())
        {
            return Err(refuse("ALLIANCE_ACTIVE", &[]));
        }
        let members = self.alliance_members(alliance).count();
        if members == 0 && !ai {
            return Err(refuse("ALLIANCE_EMPTY", &[]));
        }
        if members >= RULES.max_members {
            return Err(refuse("ALLIANCE_FULL", &[RULES.max_members.to_string()]));
        }
        let points = g.reputation.score / (crate::reputation::SCALE / 100);
        if points < RULES.min_reputation_points {
            return Err(refuse(
                "ALLIANCE_REPUTATION",
                &[points.to_string(), RULES.min_reputation_points.to_string()],
            ));
        }
        let routes = active_routes(g);
        if routes < RULES.min_routes {
            return Err(refuse(
                "ALLIANCE_SIZE",
                &[routes.to_string(), RULES.min_routes.to_string()],
            ));
        }
        let reserve = RULES.min_cash_months * monthly_fee(g);
        if g.cash() < reserve {
            return Err(refuse(
                "ALLIANCE_CASH",
                &[g.cash().to_string(), reserve.to_string()],
            ));
        }
        let (overlap, _) = self.alliance_overlap(owner, alliance);
        let limit = if ai {
            RULES.ai_max_overlap_percent
        } else {
            RULES.max_overlap_percent
        };
        if overlap > limit {
            return Err(refuse(
                "ALLIANCE_OVERLAP",
                &[overlap.to_string(), limit.to_string()],
            ));
        }
        Ok(())
    }
    pub fn apply_alliance(&mut self, alliance: &str) -> Result<(), String> {
        self.require_active()?;
        if def(alliance).is_none() {
            return Err("ALLIANCE_MISSING".into());
        }
        self.alliance_check(PLAYER, alliance).map_err(|r| r.code)?;
        self.alliances.application = Some(Application {
            alliance: alliance.into(),
            applied_at: self.clock.elapsed().as_secs(),
        });
        Ok(())
    }
    /// Withdraws a pending application, or gives notice of leaving.
    pub fn leave_alliance(&mut self) -> Result<(), String> {
        self.require_active()?;
        if self.alliances.application.take().is_some() {
            return Ok(());
        }
        let now = self.clock.elapsed().as_secs();
        let notice = now + RULES.leave_notice_days * DAY;
        let member = self
            .alliances
            .members
            .iter_mut()
            .find(|m| m.owner == PLAYER)
            .ok_or("ALLIANCE_NONE")?;
        if member.leaving_at.is_some() {
            return Err("ALLIANCE_LEAVING".into());
        }
        member.leaving_at = Some(notice);
        Ok(())
    }
    pub fn next_alliance_event(&self) -> Option<u64> {
        if self.airline.is_none() || self.insolvent() || self.competitors.is_empty() {
            return None;
        }
        Some((self.alliances.reviewed_at.max(self.clock.elapsed().as_secs()) / cycle() + 1) * cycle())
    }
    fn alliance_notice(&mut self, kind: &str, alliance: &str, text: String) {
        let id = self.alliances.next_notice;
        self.alliances.next_notice += 1;
        let at = self.clock.elapsed().as_secs();
        self.alliances.notices.push(Notice {
            id,
            at,
            kind: kind.into(),
            alliance: alliance.into(),
            text,
        });
        let excess = self.alliances.notices.len().saturating_sub(NOTICE_LIMIT);
        self.alliances.notices.drain(..excess);
    }
    fn remove_player_member(&mut self, kind: &str, why: &str) {
        let now = self.clock.elapsed().as_secs();
        let Some(i) = self.alliances.members.iter().position(|m| m.owner == PLAYER) else {
            return;
        };
        let alliance = self.alliances.members.remove(i).alliance;
        self.alliances.blocked_until = now + RULES.reapply_cooldown_days * DAY;
        let name = def(&alliance).map_or("", |d| d.name.as_str()).to_owned();
        self.alliance_notice(kind, &alliance, format!("{name}: {why}"));
    }
    fn end_rival_codeshares(&mut self) -> bool {
        let Some(mine) = self.alliance_of(PLAYER).map(|m| m.alliance.clone()) else {
            return false;
        };
        let rivals: Vec<String> = self
            .codeshares
            .iter()
            .filter(|a| {
                self.alliances
                    .members
                    .iter()
                    .any(|m| self.alliance_offer_key(&m.owner) == a.partner && m.alliance != mine)
            })
            .map(|a| a.partner.clone())
            .collect();
        self.codeshares.retain(|a| !rivals.contains(&a.partner));
        !rivals.is_empty()
    }
    /// Drops members that no longer exist or are in restructuring, and keeps the player's
    /// bilateral codeshares free of rival-alliance partners. Event-time state only.
    pub(crate) fn prune_alliances(&mut self) {
        if self.alliances.is_empty() {
            return;
        }
        let gone: Vec<String> = self
            .alliances
            .members
            .iter()
            .filter(|m| self.alliance_game(&m.owner).is_none_or(unavailable))
            .map(|m| m.owner.clone())
            .collect();
        let mut changed = !gone.is_empty();
        if gone.iter().any(|o| o == PLAYER) {
            self.remove_player_member("removed", "Mitgliedschaft beendet: Restrukturierung");
        }
        self.alliances.members.retain(|m| !gone.contains(&m.owner));
        if self.alliances.application.is_some() && unavailable(self) {
            self.alliances.application = None;
        }
        changed |= self.end_rival_codeshares();
        if changed {
            self.commercial_changed();
        }
    }
    pub fn apply_alliances(&mut self) -> Result<(), String> {
        if self.next_alliance_event().is_none() {
            return Ok(());
        }
        let now = self.clock.elapsed().as_secs();
        if !now.is_multiple_of(cycle()) || now <= self.alliances.reviewed_at {
            return Ok(());
        }
        self.prune_alliances();
        // Distressed members are dropped; AI members also leave when their network shrinks.
        let owners: Vec<String> = self.alliances.members.iter().map(|m| m.owner.clone()).collect();
        for owner in owners {
            let Some(g) = self.alliance_game(&owner) else {
                continue;
            };
            if g.distressed() {
                if owner == PLAYER {
                    self.remove_player_member("removed", "Mitgliedschaft beendet: Finanznot");
                } else {
                    self.alliances.members.retain(|m| m.owner != owner);
                }
            } else if owner != PLAYER && active_routes(g) < RULES.ai_leave_below_routes {
                self.alliances.members.retain(|m| m.owner != owner);
            }
        }
        if self
            .alliances
            .members
            .iter()
            .any(|m| m.owner == PLAYER && m.leaving_at.is_some_and(|t| t <= now))
        {
            self.remove_player_member("left", "Austritt abgeschlossen");
        }
        let paying: Vec<(String, i64)> = self
            .alliances
            .members
            .iter()
            .filter(|m| m.joined_at < now)
            .filter_map(|m| Some((m.owner.clone(), monthly_fee(self.alliance_game(&m.owner)?))))
            .collect();
        for (owner, fee) in paying {
            if let Some(g) = self.alliance_game_mut(&owner) {
                charge(g, now, fee)?;
            }
        }
        if let Some(app) = self.alliances.application.clone()
            && app.applied_at < now
        {
            self.alliances.application = None;
            let name = def(&app.alliance).map_or("", |d| d.name.as_str()).to_owned();
            match self.alliance_check(PLAYER, &app.alliance) {
                Ok(()) => {
                    self.alliances.members.push(Member {
                        owner: PLAYER.into(),
                        alliance: app.alliance.clone(),
                        joined_at: now,
                        leaving_at: None,
                    });
                    self.alliance_notice(
                        "accepted",
                        &app.alliance,
                        format!("{name}: Aufnahme bestätigt"),
                    );
                }
                Err(r) => self.alliance_notice(
                    "refused",
                    &app.alliance,
                    format!("{name}: Antrag abgelehnt: {}", r.text()),
                ),
            }
        }
        let ai_owners: Vec<String> = self
            .competitors
            .iter()
            .map(|c| c.game.market_owner_key())
            .collect();
        for owner in ai_owners {
            if self.alliance_of(&owner).is_some() {
                continue;
            }
            let mut best: Option<(usize, &str)> = None;
            let mut empty: Option<&str> = None;
            for d in &RULES.alliances {
                if self.alliance_check(&owner, &d.id).is_err() {
                    continue;
                }
                let members = self.alliance_members(&d.id).count();
                if members == 0 {
                    empty.get_or_insert(d.id.as_str());
                } else if best.is_none_or(|(n, _)| members > n) {
                    best = Some((members, d.id.as_str()));
                }
            }
            if let Some(id) = best.map(|b| b.1).or(empty) {
                self.alliances.members.push(Member {
                    owner,
                    alliance: id.into(),
                    joined_at: now,
                    leaving_at: None,
                });
            }
        }
        self.alliances
            .members
            .sort_by(|a, b| (&a.alliance, &a.owner).cmp(&(&b.alliance, &b.owner)));
        self.end_rival_codeshares();
        self.alliances.reviewed_at = now;
        self.commercial_changed();
        Ok(())
    }
    /// Shares connecting passengers between all members of an alliance and adds the
    /// small demand bonus on markets where other members already operate.
    pub(crate) fn apply_alliance_offers(&self, offers: &mut [Offer]) {
        if self.alliances.members.is_empty() {
            return;
        }
        let mut groups: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for m in &self.alliances.members {
            if self.alliance_game(&m.owner).is_some_and(|g| !unavailable(g)) {
                groups
                    .entry(m.alliance.as_str())
                    .or_default()
                    .push(self.alliance_offer_key(&m.owner));
            }
        }
        let mut served: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for o in offers.iter() {
            let set = served.entry(o.owner.as_str()).or_default();
            set.insert(o.route.origin_icao.as_str());
            set.insert(o.route.destination_icao.as_str());
        }
        let mut updates = Vec::new();
        for (i, o) in offers.iter().enumerate() {
            let Some(group) = groups.values().find(|g| g.contains(&o.owner)) else {
                continue;
            };
            let others: Vec<&String> = group.iter().filter(|k| **k != o.owner).collect();
            let strong = others
                .iter()
                .filter(|k| {
                    served.get(k.as_str()).is_some_and(|s| {
                        s.contains(o.route.origin_icao.as_str())
                            || s.contains(o.route.destination_icao.as_str())
                    })
                })
                .count() as i64;
            updates.push((
                i,
                others.into_iter().cloned().collect::<Vec<_>>(),
                (strong * RULES.demand_bonus_bps_per_partner).min(RULES.demand_bonus_max_bps),
            ));
        }
        for (i, others, bonus) in updates {
            for k in others {
                if !offers[i].codeshare_partners.contains(&k) {
                    offers[i].codeshare_partners.push(k);
                }
            }
            offers[i].alliance_bonus_bps = bonus;
        }
    }
    pub fn validate_alliances(&self) -> Result<(), String> {
        let bad = || Err("Ungültige Allianzdaten.".to_string());
        let now = self.clock.elapsed().as_secs();
        let s = &self.alliances;
        if self.competitors.iter().any(|c| !c.game.alliances.is_empty())
            || s.reviewed_at > now
            || s.blocked_until > now + RULES.reapply_cooldown_days * DAY
            || s.notices.len() > NOTICE_LIMIT
            || s.notices.windows(2).any(|w| w[0].id >= w[1].id)
            || s.notices.iter().any(|n| n.at > now || n.id >= s.next_notice)
        {
            return bad();
        }
        let mut seen = BTreeSet::new();
        for m in &s.members {
            if def(&m.alliance).is_none()
                || !seen.insert(&m.owner)
                || m.joined_at > now
                || m.leaving_at.is_some_and(|t| t < m.joined_at)
                || self.alliance_game(&m.owner).is_none()
                || (m.owner != PLAYER && m.leaving_at.is_some())
            {
                return bad();
            }
        }
        for d in &RULES.alliances {
            if self.alliance_members(&d.id).count() > RULES.max_members {
                return bad();
            }
        }
        if s.application
            .as_ref()
            .is_some_and(|a| def(&a.alliance).is_none() || a.applied_at > now)
        {
            return bad();
        }
        if let Some(h) = &self.history
            && (h
                .alliance_fees
                .iter()
                .any(|p| p.amount <= 0 || p.time < h.since || p.time > now)
                || h.alliance_fees.windows(2).any(|p| p[0].time > p[1].time))
        {
            return Err("Ungültiges Allianzarchiv.".into());
        }
        Ok(())
    }
}
