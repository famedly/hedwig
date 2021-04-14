use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::Deref;
pub mod models;
use crate::models::{Device, MatrixError, Notification};
use prometheus::core::{Atomic, AtomicU64, Collector, Desc, GenericCounterVec, GenericGauge};
use prometheus::proto::MetricFamily;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

type U64Counter = GenericCounterVec<AtomicU64>;

#[derive(Clone)]
pub struct NotificationCounter(pub U64Counter);
impl Deref for NotificationCounter {
    type Target = U64Counter;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct DeviceCounter(pub U64Counter);
impl Deref for DeviceCounter {
    type Target = U64Counter;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub enum NotificationType {
    Notification,
    Data,
    Clearing,
}

impl Display for NotificationType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                NotificationType::Notification => "notification",
                NotificationType::Data => "data",
                NotificationType::Clearing => "clearing",
            }
        )
    }
}

pub struct ProcessedNotification<'a, 'b> {
    pub notification: &'a Notification,
    first_device: &'a Device,
    app_id: &'b str,
}

impl ProcessedNotification<'_, '_> {
    // Some notifications may just inform the device that there are no more unread rooms
    pub fn is_clearing(&self) -> bool {
        self.notification.event_id.is_none()
            || (!self.is_data_message() && self.unread_count() == 0) // Even if there are 0 unread, we should let data message recipient decide what they want to do
    }

    // Whether the push gateway should send only a data message - we have a specific app_id suffix for this
    pub fn is_data_message(&self) -> bool {
        self.first_device.app_id == format!("{}.data_message", self.app_id)
    }

    pub fn r#type(&self) -> NotificationType {
        match (self.is_clearing(), self.is_data_message()) {
            (false, true) => NotificationType::Data,
            (false, false) => NotificationType::Notification,
            (true, _) => NotificationType::Clearing,
        }
    }

    pub fn device_count(&self) -> usize {
        self.notification.devices.len()
    }

    pub fn push_keys(&self) -> Vec<&String> {
        self.notification
            .devices
            .iter()
            .filter_map(|device| {
                if &device.app_id == &self.app_id
                    || device.app_id == format!("{}.data_message", &self.app_id)
                {
                    Some(&device.pushkey)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn unread_count(&self) -> u16 {
        self.notification
            .counts
            .as_ref()
            .and_then(|counts| counts.unread)
            .unwrap_or(0)
    }

    pub fn process<'a, 'b>(
        push_notification: &'a models::PushNotification,
        app_id: &'b str,
    ) -> Result<ProcessedNotification<'a, 'b>, MatrixError> {
        Ok(ProcessedNotification {
            first_device: push_notification.first_device()?,
            app_id,
            notification: &push_notification.notification,
        })
    }
}

#[derive(Clone)]
pub struct LastSuccessfulCollector {
    last_succesful_timestamp: Arc<AtomicU64>,
    collector: GenericGauge<AtomicU64>,
}

impl LastSuccessfulCollector {
    pub fn new(metric_name: &str, description: &str) -> Self {
        Self {
            last_succesful_timestamp: Arc::new(AtomicU64::new(0)),
            collector: GenericGauge::new(metric_name, description).expect("Creating a gauge"),
        }
    }

    pub fn update(&self) {
        self.last_succesful_timestamp.set(Self::current_timestamp())
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Now should be never smaller than UNIX_EPOCH")
            .as_secs()
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
                    .unwrap_or(0),
            );
            self.collector.collect()
        } else {
            vec![]
        }
    }
}
