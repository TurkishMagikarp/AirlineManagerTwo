use crate::{
    charters::{AI_DISABLED, Offer, RULES},
    game::Game,
    trading::Kind,
};
use serde_json::Value;

const DAY: u64 = 86400;

fn state(g: &Game) -> Value {
    serde_json::to_value(g).unwrap()
}
fn game(model: &str) -> Game {
    let mut g = Game::default();
    g.found("Charter Air", "DE", "EDDF").unwrap();
    g.competitors.clear();
    g.lease_aircraft(model).unwrap();
    let n = crate::staff::needs(model).unwrap();
    g.hire_pilots(&n.rating, n.pilots).unwrap();
    g.clock.set_running(true);
    g
}
fn restart(g: &Game) -> Game {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("charter.json");
    crate::save::save(&path, g).unwrap();
    crate::save::load(&path).unwrap()
}
fn offer(kind: &str, origin: &str, destination: &str, quantity: u32, now: u64) -> Offer {
    Offer {
        id: 9_000_001,
        kind: kind.into(),
        origin: origin.into(),
        destination: destination.into(),
        quantity,
        fee_eur: 100_000,
        created: now,
        window_start: now + DAY,
        window_end: now + 4 * DAY,
        expires: now + 6 * DAY,
    }
}
fn refusal(g: &Game, o: &Offer) -> String {
    g.quote_charter_offer(o, 1).unwrap_err()
}
/// Passenger offer an A320 can take, optionally starting where the aircraft is.
fn pick(g: &Game, at_aircraft: bool) -> Offer {
    let home = g.fleet[0].location_icao.clone();
    let fits: Vec<Offer> = g
        .open_charters()
        .into_iter()
        .filter(|o| g.quote_charter_offer(o, 1).is_ok())
        .collect();
    fits.iter()
        .find(|o| (o.origin == home) == at_aircraft)
        .or(fits.first())
        .expect("a suitable offer")
        .clone()
}
fn run(g: &mut Game, days: u64) {
    let speed = f64::from(g.clock.speed());
    for _ in 0..days {
        assert!(g.advance(DAY as f64 / speed));
    }
}

#[test]
fn offers_are_deterministic_seasonal_and_economy_dependent() {
    let g = game("a320_200");
    let a = crate::charters::epoch_offers(&g.world_economy, 3);
    assert_eq!(a, crate::charters::epoch_offers(&g.world_economy, 3));
    assert!(!a.is_empty());
    let mut other = g.world_economy;
    other.seed ^= 0xdead_beef;
    assert_ne!(a, crate::charters::epoch_offers(&other, 3));
    let mut summer = 0;
    let mut winter = 0;
    for e in 0..52u64 {
        let n = crate::charters::epoch_offers(&g.world_economy, e).len();
        let leisure = crate::demand::season_factors(e * RULES.epoch_days * DAY).leisure;
        if leisure > 1.05 {
            summer += n;
        } else if leisure < 0.95 {
            winter += n;
        }
    }
    assert!(summer > winter, "{summer} vs {winter}");
    for o in &a {
        assert!(o.window_start >= o.created && o.window_end > o.window_start && o.expires > o.created);
        assert!(o.origin != o.destination && o.fee_eur > 0);
        assert!(["tour_operator", "sports_team", "freight"].contains(&o.kind.as_str()));
    }
}

#[test]
fn charter_refusals_say_exactly_why() {
    let g = game("a320_200");
    let now = g.clock.elapsed().as_secs();
    let ok = offer("tour_operator", "EDDF", "EDDM", 100, now);
    assert!(g.quote_charter_offer(&ok, 1).is_ok());
    assert!(refusal(&g, &offer("tour_operator", "EDDF", "EDDM", 250, now)).contains("Sitze"));
    assert!(refusal(&g, &offer("freight", "EDDF", "EDDM", 5, now)).contains("Frachter"));
    assert!(refusal(&g, &offer("tour_operator", "EDDF", "NZAA", 100, now)).contains("Reichweite"));
    let mut expired = ok.clone();
    expired.expires = now;
    assert!(refusal(&g, &expired).contains("abgelaufen"));
    assert!(g.quote_charter_offer(&ok, 99).unwrap_err().contains("Unbekannt"));

    let mut due = g.clone();
    due.fleet[0].wear = due.fleet[0].wear.added(100_000_000, 100_000).unwrap();
    assert!(refusal(&due, &ok).contains("Wartung"));

    let mut routed = g.clone();
    routed.create_route("EDDF", "EDDM", 1).unwrap();
    assert!(refusal(&routed, &ok).contains("Route") || refusal(&routed, &ok).contains("Flugplan"));

    let mut crewless = g.clone();
    crewless.staff.employees.clear();
    assert!(!refusal(&crewless, &ok).is_empty());

    let mut taken = g.clone();
    taken.accept_charter_offer(&ok, 1).unwrap();
    assert!(refusal(&taken, &ok).contains("Charterauftrag"));

    let f = game("b737_800bcf");
    assert!(refusal(&f, &offer("tour_operator", "EDDF", "EDDM", 10, now)).contains("Frachter"));
    assert!(refusal(&f, &offer("freight", "EDDF", "EDDM", 30, now)).contains("Nutzlast"));
    assert!(f.quote_charter_offer(&offer("freight", "EDDF", "EDDM", 20, now), 1).is_ok());
}

#[test]
fn unknown_or_taken_offers_cannot_be_accepted() {
    let mut g = game("a320_200");
    let before = state(&g);
    assert!(g.accept_charter(123, 1).is_err());
    assert_eq!(state(&g), before);
    let o = pick(&g, false);
    g.accept_charter(o.id, 1).unwrap();
    assert!(g.open_charters().iter().all(|x| x.id != o.id));
    assert!(g.accept_charter(o.id, 1).is_err());
}

fn complete(at_origin: bool) {
    let mut g = game("a320_200");
    let mut o = pick(&g, at_origin);
    if at_origin {
        g.fleet[0].location_icao = o.origin.clone();
    }
    o.fee_eur = 90_000;
    let positioning = g.fleet[0].location_icao != o.origin;
    let q = g.quote_charter_offer(&o, 1).unwrap();
    assert_eq!(q.positioning.is_some(), positioning);
    g.accept_charter_offer(&o, 1).unwrap();
    g.charters.taken.push((o.id, o.expires));
    let cost = q.cost_eur();
    let cash = g.cash();
    let reputation = g.reputation.score;
    run(&mut g, 14);
    assert!(g.charters.jobs.is_empty(), "{:?}", g.charters);
    let last = g.charters.recent.last().unwrap();
    assert!(last.delivered, "{last:?}");
    assert_eq!(g.fleet[0].location_icao, o.destination);
    let end = g.clock.elapsed().as_secs();
    let h = g.history.as_ref().unwrap();
    assert_eq!(h.charter_through(end, Kind::CharterFee), 90_000);
    assert_eq!(h.ferry_through(end), i128::from(cost));
    let other = g.history.as_ref().unwrap().aircraft_cash_through(end)
        - h.charter_through(end, Kind::CharterFee)
        + h.ferry_through(end);
    assert!(other <= 0);
    assert!(g.cash() < cash + 90_000 - cost + 1);
    assert!(g.reputation.score >= reputation);
    g.validate_charters().unwrap();
}

#[test]
fn charter_without_positioning_pays_fee_and_books_costs() {
    complete(true);
}

#[test]
fn charter_with_positioning_leg_pays_fee_and_books_costs() {
    complete(false);
}

#[test]
fn failed_charter_costs_penalty_and_reputation() {
    let mut g = game("a320_200");
    let o = pick(&g, false);
    g.accept_charter_offer(&o, 1).unwrap();
    g.charters.taken.push((o.id, o.expires));
    let cash = g.cash();
    let reputation = g.reputation.score;
    g.cancel_charter(o.id).unwrap();
    assert_eq!(g.cash(), cash - o.penalty_eur());
    assert!(g.reputation.score < reputation);
    assert!(!g.charters.recent.last().unwrap().delivered);
    let t = g.clock.elapsed().as_secs();
    assert_eq!(
        g.history.as_ref().unwrap().charter_through(t, Kind::CharterPenalty),
        -i128::from(o.penalty_eur())
    );
    assert!(g.cancel_charter(o.id).is_err());

    // Selling the aircraft fails the job too.
    let mut s = game("a320_200");
    let o = pick(&s, true);
    s.fleet[0].location_icao = o.origin.clone();
    s.accept_charter_offer(&o, 1).unwrap();
    s.sell_aircraft(1).or_else(|_| s.return_aircraft(1)).unwrap();
    assert!(s.charters.jobs.is_empty());
    assert!(!s.charters.recent.last().unwrap().delivered);

    // Losing the crew before departure fails it at dispatch.
    let mut c = game("a320_200");
    let o = pick(&c, true);
    c.fleet[0].location_icao = o.origin.clone();
    c.accept_charter_offer(&o, 1).unwrap();
    c.charters.taken.push((o.id, o.expires));
    c.staff.employees.clear();
    for _ in 0..14 {
        run(&mut c, 1);
        if c.charters.jobs.is_empty() {
            break;
        }
    }
    assert!(c.charters.jobs.is_empty());
    assert!(!c.charters.recent.last().unwrap().delivered);
    c.validate_charters().unwrap();
}

#[test]
fn failed_charter_stops_a_time_jump_and_shows_attention() {
    let mut g = game("a320_200");
    let o = pick(&g, true);
    g.fleet[0].location_icao = o.origin.clone();
    g.accept_charter_offer(&o, 1).unwrap();
    g.charters.taken.push((o.id, o.expires));
    g.staff.employees.clear();
    let r = g.jump(20.0 * DAY as f64).unwrap();
    assert!(!r.reason.is_empty(), "jump should stop on the failed charter");
    assert!(g.attention_snapshot().iter().any(|e| e.kind == "charter_failed"));
}

#[test]
fn expiring_offer_that_fits_an_idle_aircraft_is_in_attention() {
    let mut g = game("a320_200");
    let o = pick(&g, false);
    let warn = RULES.expiry_warning_hours * 3600;
    g.clock = crate::clock::Clock::from_state(
        std::time::Duration::from_secs(o.expires - warn / 2),
        true,
        1,
    )
    .unwrap();
    let fits = g.open_charters().iter().any(|x| x.id == o.id);
    if fits {
        assert!(g.attention_snapshot().iter().any(|e| e.kind == "charter_expiring"));
    }
}

#[test]
fn charters_are_identical_at_all_speeds_and_across_restart_mid_flight() {
    let mut initial = game("a320_200");
    let o = pick(&initial, true);
    initial.fleet[0].location_icao = o.origin.clone();
    initial.accept_charter_offer(&o, 1).unwrap();
    initial.charters.taken.push((o.id, o.expires));
    let q_dispatch = initial.charters.jobs[0].dispatch_at;
    let mut baseline = initial.clone();
    run(&mut baseline, 14);
    assert!(baseline.charters.recent.last().unwrap().delivered);
    for speed in [1u32, 4, 16] {
        let mut g = initial.clone();
        g.clock.set_speed(speed);
        // Stop while the charter leg is airborne and restart there.
        let airborne = q_dispatch + 600;
        while g.clock.elapsed().as_secs() < airborne {
            assert!(g.advance(3600.0 / f64::from(speed)));
        }
        assert!(g.charters.jobs[0].departure.is_some(), "speed {speed}");
        g = restart(&g);
        g.clock.set_running(true);
        let done = g.clock.elapsed().as_secs();
        let target = 14 * DAY;
        let mut left = target.saturating_sub(done);
        while left > 0 {
            let step = left.min(3600);
            assert!(g.advance(step as f64 / f64::from(speed)));
            left -= step;
        }
        g.clock.set_speed(1);
        assert_eq!(state(&g)["charters"], state(&baseline)["charters"], "speed {speed}");
        assert_eq!(g.cash(), baseline.cash(), "speed {speed}");
    }
}

#[test]
fn old_saves_load_without_charters_and_offers_start_later() {
    let mut g = game("a320_200");
    run(&mut g, 10);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.json");
    crate::save::save(&path, &g).unwrap();
    let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["version"] = 45.into();
    v["state"].as_object_mut().unwrap().remove("charters");
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let l = crate::save::load(&path).unwrap();
    assert!(l.charters.jobs.is_empty() && l.charters.recent.is_empty());
    assert_eq!(l.charters.since, l.clock.elapsed().as_secs());
    let now = l.clock.elapsed().as_secs();
    assert!(l.open_charters().iter().all(|o| o.created >= now));
    let mut l = l;
    run(&mut l, 14);
    assert!(!l.open_charters().is_empty());
    l.validate_charters().unwrap();
}

#[test]
fn an_ai_airline_can_take_an_offer_before_the_player() {
    AI_DISABLED.with(|d| d.set(false));
    let mut g = Game::default();
    g.found("Charter Air", "DE", "EDDF").unwrap();
    g.clock.set_running(true);
    assert!(!g.competitors.is_empty());
    g.competitors[0].game.lease_aircraft("a320_200").unwrap();
    g.competitors[0].game.staff_up();
    let id = g.competitors[0].game.fleet.last().unwrap().id;
    assert!(g.competitors[0].game.ai_charter_candidates_len() >= 1);
    let before = g.open_charters();
    g.ai_charters(0).unwrap();
    AI_DISABLED.with(|d| d.set(true));
    assert!(!g.competitors[0].game.charters.jobs.is_empty(), "AI took nothing from {} offers", before.len());
    assert!(g.competitors[0].game.charters.jobs.iter().any(|j| j.aircraft_id == id));
    let player_offers = g.open_charters();
    assert!(player_offers.len() < before.len());
    let ids: Vec<u64> = g
        .competitors
        .iter()
        .flat_map(|c| c.game.charters.jobs.iter().map(|j| j.offer.id).chain(c.game.charters.recent.iter().map(|o| o.offer_id)))
        .collect();
    assert!(g.charters.taken.iter().any(|t| ids.contains(&t.0)));
    assert!(g.open_charters().iter().all(|o| !ids.contains(&o.id)));
    // No two airlines hold the same offer.
    let mut jobs: Vec<u64> = g.competitors.iter().flat_map(|c| c.game.charters.jobs.iter().map(|j| j.offer.id)).collect();
    jobs.extend(g.charters.jobs.iter().map(|j| j.offer.id));
    let n = jobs.len();
    jobs.sort();
    jobs.dedup();
    assert_eq!(n, jobs.len());
}

fn ai_run(charters: bool, country: &str, home: &str, years: u32, spare: bool) -> Vec<String> {
    AI_DISABLED.with(|d| d.set(!charters));
    let mut g = Game::default();
    g.found("Charter audit", country, home).unwrap();
    g.clock.set_running(true);
    if spare {
        g.competitors[0].game.lease_aircraft("a320_200").unwrap();
        g.competitors[0].game.staff_up();
    }
    for _ in 0..(years * 52) {
        assert!(g.advance(7.0 * DAY as f64));
    }
    AI_DISABLED.with(|d| d.set(true));
    g.competitors
        .iter()
        .map(|c| {
            let h = c.game.history.as_ref().unwrap();
            let fees = h.charter_through(u64::MAX, Kind::CharterFee);
            let penalties = h.charter_through(u64::MAX, Kind::CharterPenalty);
            let revenue = c.game.finance.totals.revenue;
            format!(
                "{} cash {} fleet {} fees {fees} penalties {penalties} total_revenue {revenue} insolvent {}",
                c.game.market_owner_key(),
                c.game.cash(),
                c.game.fleet.len(),
                c.game.insolvent()
            )
        })
        .collect()
}

#[test]
#[ignore = "explicit ten-year audit of AI finances with and without charters"]
fn charters_ai_finance_audit() {
    for (country, home) in [("DE", "EDDF")] {
        let with = ai_run(true, country, home, 10, false);
        let without = ai_run(false, country, home, 10, false);
        for (a, b) in with.iter().zip(&without) {
            eprintln!("{home} WITH    {a}\n{home} WITHOUT {b}");
        }
    }
}

#[test]
#[ignore = "explicit audit: one AI airline with a spare idle aircraft"]
fn charters_ai_spare_aircraft_audit() {
    for years in [2, 6] {
        for charters in [true, false] {
            for line in ai_run(charters, "DE", "EDDF", years, true) {
                eprintln!("years {years} charters {charters}: {line}");
            }
        }
    }
}

/// Hands competitor 0 a crewed, idle A320 and lets it review the open offers.
fn with_crewed_spare(g: &Game) -> (Game, u32) {
    let mut h = g.clone();
    let ai = &mut h.competitors[0].game;
    ai.lease_aircraft("a320_200").unwrap();
    let id = ai.fleet.last().unwrap().id;
    ai.equip_staff_for("a320_200").unwrap();
    for e in &mut ai.staff.employees {
        e.training = None;
    }
    let n = crate::staff::needs("a320_200").unwrap();
    assert!(ai.rated_available(&n.rating) >= n.pilots, "spare must be crewed");
    (h, id)
}

#[test]
fn an_ai_with_a_crewed_idle_aircraft_earns_charter_income_over_two_years() {
    AI_DISABLED.with(|d| d.set(true));
    let mut g = Game::default();
    g.found("Charter audit", "DE", "EDDF").unwrap();
    g.clock.set_running(true);
    let mut taken = 0;
    let mut earned = None;
    for _ in 0..104 {
        assert!(g.advance(7.0 * DAY as f64));
        if g.competitors[0].game.distressed() {
            continue;
        }
        let (mut h, id) = with_crewed_spare(&g);
        AI_DISABLED.with(|d| d.set(false));
        h.ai_charters(0).unwrap();
        AI_DISABLED.with(|d| d.set(true));
        if h.competitors[0].game.charter_job(id).is_some() {
            taken += 1;
            if earned.is_none() {
                // Fly it out (AI reviews stay off the charter path while the job runs).
                for _ in 0..3 {
                    assert!(h.advance(7.0 * DAY as f64));
                }
                let fees = h.competitors[0]
                    .game
                    .history
                    .as_ref()
                    .unwrap()
                    .charter_through(u64::MAX, Kind::CharterFee);
                earned = Some(fees);
            }
        }
    }
    eprintln!("weeks with a taken charter: {taken}/104, first charter fee income {earned:?}");
    assert!(taken > 0, "a crewed idle aircraft never found a paying charter in two years");
    assert!(earned.unwrap() > 0, "the taken charter paid no fee");
}

#[test]
#[ignore = "explicit audit: why AI airlines rarely fly charters (funnel per weekly review)"]
fn charters_ai_idle_funnel_audit() {
    AI_DISABLED.with(|d| d.set(true));
    let mut g = Game::default();
    g.found("Charter audit", "DE", "EDDF").unwrap();
    g.clock.set_running(true);
    let (mut ticks, mut idle_ticks, mut quotes) = (0, 0, 0);
    let mut why = std::collections::BTreeMap::<String, u32>::new();
    for _ in 0..(10 * 52) {
        assert!(g.advance(7.0 * DAY as f64));
        let offers = g.open_charters();
        for c in &g.competitors {
            ticks += 1;
            let ids = c.game.ai_charter_candidates();
            if ids.is_empty() {
                continue;
            }
            idle_ticks += 1;
            // Quote as if crewed: the crew block alone is reported separately.
            let mut crewed = c.game.clone();
            for id in &ids {
                let model = crewed.fleet.iter().find(|p| p.id == *id).unwrap().model_id.clone();
                crewed.equip_staff_for(&model).unwrap();
            }
            for e in &mut crewed.staff.employees {
                e.training = None;
            }
            for id in ids {
                let blocked = c.game.quote_charter_offer(&offers[0], id).is_err_and(|e| e.starts_with("Berechtigte Piloten"));
                *why.entry(format!("[aircraft without free crew: {blocked}]")).or_default() += 1;
                for o in &offers {
                    quotes += 1;
                    let key = match crewed.quote_charter_offer(o, id) {
                        Err(e) => e.split(':').next().unwrap().to_string(),
                        Ok(q) => {
                            let net = o.fee_eur - q.cost_eur();
                            if net >= RULES.ai_min_net_eur && net * 100 >= q.cost_eur() * RULES.ai_min_margin_percent {
                                "ACCEPTABLE".into()
                            } else if net > 0 {
                                "profit below the AI thresholds".into()
                            } else {
                                "fee below cost".into()
                            }
                        }
                    };
                    *why.entry(key).or_default() += 1;
                }
            }
        }
    }
    eprintln!("ten years: {ticks} airline-weeks, {idle_ticks} with an idle aircraft, {quotes} aircraft/offer pairs");
    for (k, n) in &why {
        eprintln!("  {n:6} {k}");
    }
}
