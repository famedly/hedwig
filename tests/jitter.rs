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

#![allow(
	clippy::dbg_macro,
	clippy::expect_used,
	clippy::missing_docs_in_private_items,
	clippy::print_stderr,
	clippy::print_stdout,
	clippy::unwrap_used
)]

use std::time::Duration;

use matrix_hedwig::jitter;

#[test]
fn jitter_test() {
	// Examples from https://github.com/Famedly/matrix-doc/blob/jcgruenhage/delayed-push/proposals/3359-delayed-push.md#recommended-values-of-random_delay
	for (i, o) in [
		(0.25, 13.656_854_249_492_383),
		(0.5, 6.828_427_124_746_192),
		(1.0, 3.414_213_562_373_096),
		(10.0, 0.341_421_356_237_309_5),
		(100.0, 0.034_142_135_623_730_96),
	] {
		assert_eq!(jitter::Jitter::jitter(i), Duration::from_secs_f64(o));
	}
}
