//! Prometheus metrics

use std::{ops::Deref, sync::Arc};

use prometheus::{
	core::{Atomic, AtomicI64, AtomicU64, Collector, Desc, GenericCounterVec, GenericGauge},
	proto::MetricFamily,
};

type U64Counter = GenericCounterVec<AtomicU64>;

/// The notification counter for prometheus
#[derive(Clone, Debug)]
pub struct NotificationCounter(pub U64Counter);
impl Deref for NotificationCounter {
	type Target = U64Counter;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// The device counter for prometheus
#[derive(Clone, Debug)]
pub struct DeviceCounter(pub U64Counter);
impl Deref for DeviceCounter {
	type Target = U64Counter;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Collector of last successful notification timestamps
#[derive(Clone, Debug)]
pub struct LastSuccessfulCollector {
	/// The timestamp of last successful notification
	last_succesful_timestamp: Arc<AtomicI64>,
	/// Collector
	collector: GenericGauge<AtomicI64>,
}

impl LastSuccessfulCollector {
	/// Last successful notification collector constructor
	pub fn new(metric_name: &str, description: &str) -> Result<Self, prometheus::Error> {
		Ok(Self {
			last_succesful_timestamp: Arc::new(AtomicI64::new(0)),
			collector: GenericGauge::new(metric_name, description)?,
		})
	}

	/// Update the last successful notification timestamp
	pub fn update(&self) {
		self.last_succesful_timestamp.set(Self::current_timestamp())
	}

	/// Get the current timestamp
	fn current_timestamp() -> i64 {
		chrono::Utc::now().timestamp()
	}
}

impl Collector for LastSuccessfulCollector {
	fn desc(&self) -> Vec<&Desc> {
		self.collector.desc()
	}

	fn collect(&self) -> Vec<MetricFamily> {
		let last_timestamp = self.last_succesful_timestamp.get();
		if last_timestamp > 0 {
			self.collector.set(
				Self::current_timestamp()
					.checked_sub(self.last_succesful_timestamp.get())
					.unwrap_or_default(),
			);
			self.collector.collect()
		} else {
			vec![]
		}
	}
}
