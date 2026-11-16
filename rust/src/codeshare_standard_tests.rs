use crate::game::Game;
pub(super) fn standard_eddf_game() -> Game {
    let mut g = Game::default();
    g.found("EDDF audit", "DE", "EDDF").unwrap();
    g.buy_aircraft("atr72_600").unwrap();
    g.staff_up();
    g.create_route("EDDF", "EDDM", 1).unwrap();
    g.set_ticket_price(1, 90).unwrap();
    g.schedule(1, 1, 120).unwrap();
    g.clock.set_running(true);
    g
}
#[test]
#[ignore = "explicit unchanged EDDF two/ten-year audit"]
fn codeshare_standard_eddf_comparison() {
    let mut g = standard_eddf_game();
    for (years, days) in [(2, 730), (10, 2920)] {
        assert!(g.advance(f64::from(days) * 86400.0));
        for owner in std::iter::once(&g).chain(g.competitors.iter().map(|c| &*c.game)) {
            println!(
                "STANDARD_EDDF years={years} owner={} pax={} cash={} revenue={}",
                owner.airline.as_ref().unwrap().name,
                owner.finance.totals.passengers,
                owner.cash(),
                owner.finance.totals.revenue
            );
            assert!(!owner.distressed() && !owner.insolvent());
        }
    }
}
