use crate::{Error, Result, fail};
use globset::GlobBuilder;
use regex_automata::{
    Anchored, Input,
    dfa::{Automaton, dense},
    util::{primitives::StateID, syntax},
};
use std::collections::{HashSet, VecDeque};

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.:/-";
const MAX_LENGTH: usize = 128;
const MAX_STATES: usize = 16_384;

pub(crate) struct Scope(dense::DFA<Vec<u32>>);

impl Scope {
    pub(crate) fn new(pattern: &str) -> Result<Self> {
        if pattern.is_empty()
            || pattern.len() > MAX_LENGTH
            || !pattern.bytes().all(|byte| (b'!'..=b'~').contains(&byte))
        {
            return fail("check must be a nonempty ASCII glob of at most 128 characters.");
        }
        let glob = GlobBuilder::new(pattern)
            .literal_separator(false)
            .backslash_escape(true)
            .build()
            .map_err(|error| Error(format!("check must be a valid glob: {error}")))?;
        let dfa = dense::Builder::new()
            .configure(
                dense::Config::new()
                    .dfa_size_limit(Some(1024 * 1024))
                    .determinize_size_limit(Some(1024 * 1024)),
            )
            .syntax(syntax::Config::new().utf8(false))
            .build(glob.regex())
            .map_err(|error| Error(format!("Cannot compile check scope: {error}")))?;
        let scope = Self(dfa);
        if !scope.covers(&scope)? {
            return fail("check must match at least one valid named check identifier.");
        }
        Ok(scope)
    }

    fn start(&self) -> Result<StateID> {
        self.0
            .start_state_forward(&Input::new(b"").anchored(Anchored::Yes))
            .map_err(|error| Error(error.to_string()))
    }

    fn accepts(&self, state: StateID) -> bool {
        self.0.is_match_state(self.0.next_eoi_state(state))
    }

    pub(crate) fn covers(&self, gate: &Self) -> Result<bool> {
        let mut pending = VecDeque::from([(self.start()?, gate.start()?, 0)]);
        let mut seen = HashSet::new();
        let mut nonempty = false;
        // A counterexample is a valid check accepted by the gate but not the claim.
        // Breadth-first traversal retains the shortest path to each state pair.
        while let Some((claim, requested, length)) = pending.pop_front() {
            if length > 0 && gate.accepts(requested) {
                nonempty = true;
                if !self.accepts(claim) {
                    return Ok(false);
                }
            }
            if length == MAX_LENGTH {
                continue;
            }
            for &byte in ALPHABET {
                if length == 0 && !byte.is_ascii_alphanumeric() {
                    continue;
                }
                let requested = gate.0.next_state(requested, byte);
                if gate.0.is_dead_state(requested) {
                    continue;
                }
                let claim = self.0.next_state(claim, byte);
                if seen.insert((claim, requested)) {
                    if seen.len() > MAX_STATES {
                        return fail("Check scope comparison exceeded its state limit.");
                    }
                    pending.push_back((claim, requested, length + 1));
                }
            }
        }
        Ok(nonempty)
    }
}
