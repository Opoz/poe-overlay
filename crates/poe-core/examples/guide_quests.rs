//! Lists every quest hand-in in the Guide with its NPC and book.
use poe_core::guide::{Fragment, GuideBook, GuideOptions};
fn main() {
    let book = GuideBook::bundled();
    for s in &book.guide(GuideOptions::default()).steps {
        for l in &s.lines {
            for p in &l.parts {
                if let Fragment::Quest {
                    name, npc, book, ..
                } = p
                {
                    println!(
                        "A{} {:<28} {:<18} {:?}",
                        s.act,
                        name,
                        npc.as_deref().unwrap_or("?"),
                        book
                    );
                }
            }
        }
    }
}
