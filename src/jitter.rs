//! Implements jittering based on the delayed push msc proposal
//! https://github.com/Famedly/matrix-doc/blob/jcgruenhage/delayed-push/proposals/3359-delayed-push.md#recommended-values-of-random_delay

/*
 *   Matrix Hedwig
 *   Copyright (C) 2019, 2020, 2021, 2022 Famedly GmbH
 *
 *   This program is free software: you can redistribute it and/or modify
 *   it under the terms of the GNU Affero General Public License as
 *   published by the Free Software Foundation, either version 3 of the
 *   License, or (at your option) any later version.
 *
 *   This program is distributed in the hope that it will be useful,
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *   GNU Affero General Public License for more details.
 *
 *   You should have received a copy of the GNU Affero General Public License
 *   along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
	cmp::Reverse,
	collections::BinaryHeap,
	time::{Duration, Instant},
};

use rand::{thread_rng, Rng};

/// Struct for keeping track of frequency of requests and calculating jitter
/// delays based of it
#[derive(Debug)]
pub struct Jitter {
	/// Binary heap for sorted timestamps
	/// Since new elements may be pushed out of order and we need to keep track
	/// of the lowest timestamp this is a solution
	past_jitters: BinaryHeap<Reverse<Instant>>,
	/// Maximum amount of time a jitter is allowed to take
	max_jitter: Duration,
}

impl Jitter {
	/// Constructs a new jitter struct from the max duration that will be
	/// allowed for jittering It will automatically keep track of how much to
	/// jitter
	#[must_use]
	pub fn new(max_jitter: Duration) -> Self {
		Jitter { past_jitters: BinaryHeap::new(), max_jitter }
	}

	/// Generates jitter from frequency based on the proposed jitter msc
	/// https://github.com/Famedly/matrix-doc/blob/jcgruenhage/delayed-push/proposals/3359-delayed-push.md#recommended-values-of-random_delay
	#[must_use]
	pub fn jitter(freq: f64) -> Duration {
		let a = (2.0 - 2.0_f64.sqrt()) / 2.0;
		Duration::from_secs_f64(1.0 / (freq * a))
	}

	/// Call this function after successfully pushing a message
	/// This can't be done in get_jitter_delay since that would allow a
	/// malicious party to reduce the jitter by sending a bunch of invalid
	/// requests
	pub fn push_successful_jitter(&mut self, when: Instant) {
		self.past_jitters.push(Reverse(when));

		// sample last 25 requests for average frequency calculation
		if self.past_jitters.len() > 25 {
			self.past_jitters.pop();
		}
	}

	/// Gets a random jitter delay based on the current frequency of requests
	#[must_use]
	pub fn get_jitter_delay(&self) -> Duration {
		// TODO: 4 is chosen without deep reasoning rn
		// Do we even want to jitter this aggressively right after startup?
		let mut jitter = if self.past_jitters.len() < 4 {
			// TODO: is this a sane starting frequency?
			Self::jitter(0.25)
		} else {
			self.past_jitters.peek().map_or(self.max_jitter, |f| {
				Self::jitter(self.past_jitters.len() as f64 / f.0.elapsed().as_secs_f64())
			})
		};

		if jitter > self.max_jitter {
			jitter = self.max_jitter;
		}

		thread_rng().gen_range(Duration::default()..=jitter)
	}
}
