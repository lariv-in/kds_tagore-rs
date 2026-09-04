//! SeaORM-persistable wrapper around [`chrono::Duration`] (bigint nanoseconds).

use chrono::Duration;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, ValueType, ValueTypeErr};
use sea_orm::{ColIdx, TryGetable};
use serde::{Deserialize, Serialize};

/// Job duration stored as nanoseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDuration(pub Duration);

impl JobDuration {
    pub fn from_nanos(nanos: i64) -> Self {
        Self(Duration::nanoseconds(nanos))
    }

    pub fn num_nanoseconds(self) -> i64 {
        self.0.num_nanoseconds().unwrap_or(0)
    }

    pub fn inner(self) -> Duration {
        self.0
    }
}

impl Default for JobDuration {
    fn default() -> Self {
        Self(Duration::zero())
    }
}

impl From<Duration> for JobDuration {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl From<JobDuration> for Duration {
    fn from(value: JobDuration) -> Self {
        value.0
    }
}

impl From<JobDuration> for Value {
    fn from(source: JobDuration) -> Self {
        Value::BigInt(Some(source.num_nanoseconds()))
    }
}

impl TryGetable for JobDuration {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        i64::try_get_by(res, idx).map(Self::from_nanos)
    }
}

impl ValueType for JobDuration {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <i64 as ValueType>::try_from(v).map(Self::from_nanos)
    }

    fn type_name() -> String {
        "JobDuration".to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::BigInt
    }

    fn column_type() -> ColumnType {
        ColumnType::BigInteger
    }
}

impl Nullable for JobDuration {
    fn null() -> Value {
        Value::BigInt(None)
    }
}
