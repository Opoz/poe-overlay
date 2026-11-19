//! Prints each Guide variant's Step count and where a Step id lands.
use poe_core::guide::{BanditChoice, GuideBook, GuideOptions};
fn main() {
    let id: u32 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(10050);
    let book = GuideBook::bundled();
    for league_start in [true, false] {
        for bandit in [
            BanditChoice::KillAll,
            BanditChoice::Alira,
            BanditChoice::Kraityn,
            BanditChoice::Oak,
        ] {
            let g = book.guide(GuideOptions {
                league_start,
                bandit,
            });
            let i = g.index_of(id);
            println!(
                "league_start={league_start} {bandit:?}: step {} of {} -> {} {}",
                i + 1,
                g.steps.len(),
                g.steps[i].id,
                g.steps[i].area_name
            );
        }
    }
}
