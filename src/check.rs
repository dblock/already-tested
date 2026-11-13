use crate::{Error, Result, fail};
use globset::GlobBuilder;
use regex_automata::{
    Anchored, Input,
    dfa::{Automaton, dense},
    util::syntax,
};
use std::collections::{HashSet, VecDeque};

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.:/-";
const MAX_STATES: usize = 16_384;

pub(crate) struct Scope(dense::DFA<Vec<u32>>);

impl Scope {
    pub(crate) fn new(pattern: &str) -> Result<Self> {
        if pattern.is_empty()
            || pattern.len() > 128
            || !pattern.is_ascii()
            || pattern
                .bytes()
                .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
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

    pub(crate) fn covers(&self, gate: &Self) -> Result<bool> {
        let input = Input::new(b"").anchored(Anchored::Yes);
        let claim_start = self
            .0
            .start_state_forward(&input)
            .map_err(|error| Error(error.to_string()))?;
        let gate_start = gate
            .0
            .start_state_forward(&input)
            .map_err(|error| Error(error.to_string()))?;
        let mut pending = VecDeque::from([(claim_start, gate_start, 0)]);
        let mut seen = HashSet::new();
        let mut nonempty = false;
        // A counterexample is a valid check accepted by the gate but not the claim.
        // Breadth-first traversal retains the shortest path to each state pair.
        while let Some((claim, requested, length)) = pending.pop_front() {
            if length > 0 && gate.0.is_match_state(gate.0.next_eoi_state(requested)) {
                nonempty = true;
                if !self.0.is_match_state(self.0.next_eoi_state(claim)) {
                    return Ok(false);
                }
            }
            if length == 128 {
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
