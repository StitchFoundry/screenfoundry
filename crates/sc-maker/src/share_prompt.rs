//! When to offer "Share it on MakerFoundry?" after an export (add-on 37). Pure state: no network, no
//! window. The host asks [`should_offer`] after a successful export of separations or films, shows
//! the prompt if it says yes, and records the answer with [`SharePrompt::record`].
//!
//! The rules: at most once per project; a "Don't ask again" tick stops it everywhere; the Settings
//! toggle (MakerFoundry > "Offer to share after export") turns it off; and it never offers when the
//! export failed, so the prompt can neither block nor delay an export.

/// What the user chose in the prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Share free: open the MakerFoundry draft page.
    ShareFree,
    /// Sell it: open the draft page, where price and Seller Terms are set.
    Sell,
    /// Not now (or the prompt was dismissed).
    NotNow,
}

/// Whether the prompt has been shown for a project, kept in the project's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SharePrompt {
    /// The prompt was shown for this project (whatever the answer).
    pub shown: bool,
}

/// What the prompt's answer changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The new state of the project.
    pub prompt: SharePrompt,
    /// The Settings toggle's new value ("Don't ask again" turns it off).
    pub offer_setting: bool,
    /// Whether the host should go on to sign in and open the draft page.
    pub proceed: bool,
}

/// Whether to show the prompt now: the export `succeeded`, the Settings toggle `offer_setting` is
/// on, and this project hasn't been asked yet.
pub fn should_offer(offer_setting: bool, state: SharePrompt, export_succeeded: bool) -> bool {
    offer_setting && export_succeeded && !state.shown
}

impl SharePrompt {
    /// Record the answer. The project counts as asked whatever the choice; `dont_ask_again` turns
    /// the Settings toggle off. Only Share free and Sell go on to sign-in and the draft page.
    pub fn record(self, choice: Choice, dont_ask_again: bool, offer_setting: bool) -> Outcome {
        Outcome { prompt: SharePrompt { shown: true }, offer_setting: offer_setting && !dont_ask_again, proceed: choice != Choice::NotNow }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_on_first_successful_export() {
        assert!(should_offer(true, SharePrompt::default(), true));
    }

    #[test]
    fn once_per_project_whatever_the_answer() {
        for c in [Choice::ShareFree, Choice::Sell, Choice::NotNow] {
            let o = SharePrompt::default().record(c, false, true);
            assert!(o.prompt.shown);
            assert!(o.offer_setting);
            assert!(!should_offer(o.offer_setting, o.prompt, true));
            // A different project is still asked.
            assert!(should_offer(o.offer_setting, SharePrompt::default(), true));
        }
    }

    #[test]
    fn dont_ask_again_turns_the_setting_off_for_every_project() {
        let o = SharePrompt::default().record(Choice::NotNow, true, true);
        assert!(!o.offer_setting);
        assert!(!should_offer(o.offer_setting, SharePrompt::default(), true));
    }

    #[test]
    fn setting_off_never_offers() {
        assert!(!should_offer(false, SharePrompt::default(), true));
    }

    #[test]
    fn failed_export_never_offers() {
        assert!(!should_offer(true, SharePrompt::default(), false));
    }

    #[test]
    fn cancel_paths_do_not_proceed() {
        assert!(!SharePrompt::default().record(Choice::NotNow, false, true).proceed);
        assert!(SharePrompt::default().record(Choice::ShareFree, false, true).proceed);
        assert!(SharePrompt::default().record(Choice::Sell, false, true).proceed);
    }

    #[test]
    fn dont_ask_again_stays_off_once_off() {
        let o = SharePrompt::default().record(Choice::Sell, false, false);
        assert!(!o.offer_setting);
    }
}
