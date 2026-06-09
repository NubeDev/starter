//! The alert state machine, as a pure function.
//!
//! Given the current state, whether the latest evaluation is breaching, and
//! whether the pending dwell has elapsed, it returns the next state and the
//! transition (if any) that should be recorded and notified. No I/O lives here,
//! so the whole machine is exhaustively unit-tested without a database — the
//! evaluator handles persistence and notification around it.
//!
//! States: ok → pending → firing → resolved → ok. A rule notifies only on the
//! `Firing` and `Resolved` transitions, which is the structural dedup: a rule
//! breaching for an hour fires once, not every tick.

/// The persisted state of a rule's machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ok,
    Pending,
    Firing,
    Resolved,
}

impl State {
    /// Parse the stored string form. An unknown value is treated as `Ok` so a
    /// corrupt row fails safe (it cannot get stuck firing).
    pub fn parse(s: &str) -> Self {
        match s {
            "pending" => State::Pending,
            "firing" => State::Firing,
            "resolved" => State::Resolved,
            _ => State::Ok,
        }
    }

    /// The stored string form.
    pub fn as_str(self) -> &'static str {
        match self {
            State::Ok => "ok",
            State::Pending => "pending",
            State::Firing => "firing",
            State::Resolved => "resolved",
        }
    }
}

/// A transition worth recording and notifying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Firing,
    Resolved,
}

impl Transition {
    pub fn as_str(self) -> &'static str {
        match self {
            Transition::Firing => "firing",
            Transition::Resolved => "resolved",
        }
    }
}

/// The outcome of one evaluation step: where the rule moves to, whether that is
/// a change from the prior state (so the evaluator knows to reset the dwell
/// clock), and any transition to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub next: State,
    pub changed: bool,
    pub transition: Option<Transition>,
}

/// Advance the machine one step.
///
/// - `current` — the rule's last persisted state.
/// - `breaching` — whether this evaluation crossed the threshold.
/// - `dwell_elapsed` — whether the rule has been pending at least its `for`
///   duration (always true when `for` is zero).
///
/// `resolved` is a one-tick annotation: the next evaluation moves it back to
/// `ok` (if cleared) or straight to `firing`/`pending` (if breaching again),
/// so a recovered rule does not linger.
pub fn step(current: State, breaching: bool, dwell_elapsed: bool) -> Step {
    match (current, breaching) {
        (State::Ok, false) => keep(State::Ok),
        (State::Ok, true) => {
            // Breach begins. If there is no dwell, fire immediately; otherwise
            // wait in pending.
            if dwell_elapsed {
                change(State::Firing, Some(Transition::Firing))
            } else {
                change(State::Pending, None)
            }
        }
        (State::Pending, false) => change(State::Ok, None), // cleared before firing
        (State::Pending, true) => {
            if dwell_elapsed {
                change(State::Firing, Some(Transition::Firing))
            } else {
                keep(State::Pending)
            }
        }
        (State::Firing, true) => keep(State::Firing), // already firing — the dedup
        (State::Firing, false) => change(State::Resolved, Some(Transition::Resolved)),
        // Resolved is terminal-for-one-tick: re-derive from the fresh reading.
        (State::Resolved, false) => change(State::Ok, None),
        (State::Resolved, true) => {
            if dwell_elapsed {
                change(State::Firing, Some(Transition::Firing))
            } else {
                change(State::Pending, None)
            }
        }
    }
}

fn keep(s: State) -> Step {
    Step {
        next: s,
        changed: false,
        transition: None,
    }
}

fn change(s: State, t: Option<Transition>) -> Step {
    Step {
        next: s,
        changed: true,
        transition: t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_dwell_fires_on_first_breach_and_resolves_on_clear() {
        let s = step(State::Ok, true, true);
        assert_eq!(s.next, State::Firing);
        assert_eq!(s.transition, Some(Transition::Firing));

        // Stays firing without re-notifying.
        let s = step(State::Firing, true, true);
        assert_eq!(s.next, State::Firing);
        assert_eq!(s.transition, None);

        // Clears → resolved (notify once), then back to ok.
        let s = step(State::Firing, false, true);
        assert_eq!(s.next, State::Resolved);
        assert_eq!(s.transition, Some(Transition::Resolved));
        let s = step(State::Resolved, false, true);
        assert_eq!(s.next, State::Ok);
        assert_eq!(s.transition, None);
    }

    #[test]
    fn dwell_holds_in_pending_until_elapsed() {
        // Breach but dwell not yet elapsed → pending, no notify.
        let s = step(State::Ok, true, false);
        assert_eq!(s.next, State::Pending);
        assert_eq!(s.transition, None);

        // Still pending while not elapsed.
        let s = step(State::Pending, true, false);
        assert_eq!(s.next, State::Pending);
        assert!(!s.changed);

        // Dwell elapses → fire.
        let s = step(State::Pending, true, true);
        assert_eq!(s.next, State::Firing);
        assert_eq!(s.transition, Some(Transition::Firing));
    }

    #[test]
    fn transient_spike_in_pending_clears_without_firing() {
        let s = step(State::Pending, false, false);
        assert_eq!(s.next, State::Ok);
        assert_eq!(s.transition, None, "never fired, so nothing to resolve");
    }

    #[test]
    fn resolved_can_refire_immediately_if_breaching_again() {
        let s = step(State::Resolved, true, true);
        assert_eq!(s.next, State::Firing);
        assert_eq!(s.transition, Some(Transition::Firing));
    }

    #[test]
    fn state_string_round_trips_and_unknown_is_ok() {
        for st in [State::Ok, State::Pending, State::Firing, State::Resolved] {
            assert_eq!(State::parse(st.as_str()), st);
        }
        assert_eq!(State::parse("garbage"), State::Ok);
    }
}
