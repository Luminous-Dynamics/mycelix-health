use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::types::{
    CanonicalId, Digest32Algorithm, GeographicPrecision, MetricKind, SignalFamily, SourceKind,
    SurveillanceObservation,
};

pub const OBSERVATION_ID_DOMAIN_V1: &[u8] = b"mycelix-health-surveillance-observation-v1\0";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ObservationId([u8; 32]);

impl ObservationId {
    pub(crate) fn from_observation(observation: &SurveillanceObservation) -> Self {
        let mut h = Sha256::new();
        h.update(OBSERVATION_ID_DOMAIN_V1);
        put_u16(&mut h, observation.schema_version);
        put_signal(&mut h, &observation.signal);
        put_source_kind(&mut h, &observation.source_kind);
        put_id(&mut h, &observation.source_instance);
        put_id(&mut h, &observation.independence_group.0);
        put_id(&mut h, &observation.geography.scheme);
        put_id(&mut h, &observation.geography.code);
        put_geo_precision(&mut h, observation.geography.precision);
        put_i64(&mut h, observation.window.start_unix_s);
        put_i64(&mut h, observation.window.end_unix_s);
        put_i64(&mut h, observation.reported_at_unix_s);
        put_u64(&mut h, observation.cohort_size);
        put_metric_kind(&mut h, &observation.metric.kind);
        put_f64(&mut h, observation.metric.estimate);
        put_f64(&mut h, observation.metric.uncertainty.lower);
        put_f64(&mut h, observation.metric.uncertainty.upper);
        put_id(&mut h, &observation.metric.unit);
        put_id(&mut h, &observation.provenance.producer);
        put_id(&mut h, &observation.provenance.acquisition_protocol);
        put_id(&mut h, &observation.provenance.source_revision);
        match &observation.provenance.upstream_set {
            Some(id) => {
                h.update([1]);
                put_id(&mut h, id);
            }
            None => h.update([0]),
        }
        put_digest_algorithm(
            &mut h,
            &observation.provenance.source_record_digest.algorithm,
        );
        h.update(observation.provenance.source_record_digest.bytes);
        Self(h.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            use fmt::Write as _;
            write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
        }
        out
    }
}

impl fmt::Display for ObservationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

fn put_u16(h: &mut Sha256, value: u16) {
    h.update(value.to_be_bytes());
}

fn put_u64(h: &mut Sha256, value: u64) {
    h.update(value.to_be_bytes());
}

fn put_i64(h: &mut Sha256, value: i64) {
    h.update(value.to_be_bytes());
}

fn put_f64(h: &mut Sha256, value: f64) {
    let bits = if value == 0.0 {
        0.0f64.to_bits()
    } else {
        value.to_bits()
    };
    h.update(bits.to_be_bytes());
}

fn put_id(h: &mut Sha256, value: &CanonicalId) {
    let bytes = value.as_str().as_bytes();
    h.update((bytes.len() as u32).to_be_bytes());
    h.update(bytes);
}

fn put_signal(h: &mut Sha256, signal: &SignalFamily) {
    match signal {
        SignalFamily::Respiratory => h.update([0]),
        SignalFamily::Gastrointestinal => h.update([1]),
        SignalFamily::Febrile => h.update([2]),
        SignalFamily::Neurological => h.update([3]),
        SignalFamily::Dermatologic => h.update([4]),
        SignalFamily::Other(id) => {
            h.update([255]);
            put_id(h, id);
        }
    }
}

fn put_source_kind(h: &mut Sha256, kind: &SourceKind) {
    match kind {
        SourceKind::ClinicalSyndromicAggregate => h.update([0]),
        SourceKind::LaboratoryAggregate => h.update([1]),
        SourceKind::WastewaterAggregate => h.update([2]),
        SourceKind::EnvironmentalAggregate => h.update([3]),
        SourceKind::AbsenteeismAggregate => h.update([4]),
        SourceKind::HealthSystemCapacityAggregate => h.update([5]),
        SourceKind::Other(id) => {
            h.update([255]);
            put_id(h, id);
        }
    }
}

fn put_digest_algorithm(h: &mut Sha256, algorithm: &Digest32Algorithm) {
    match algorithm {
        Digest32Algorithm::Sha256 => h.update([0]),
        Digest32Algorithm::Blake3 => h.update([1]),
        Digest32Algorithm::Other(id) => {
            h.update([255]);
            put_id(h, id);
        }
    }
}

fn put_geo_precision(h: &mut Sha256, precision: GeographicPrecision) {
    h.update([match precision {
        GeographicPrecision::Country => 0,
        GeographicPrecision::Region => 1,
        GeographicPrecision::District => 2,
        GeographicPrecision::Facility => 3,
    }]);
}

fn put_metric_kind(h: &mut Sha256, kind: &MetricKind) {
    match kind {
        MetricKind::Count => h.update([0]),
        MetricKind::RatePer100k => h.update([1]),
        MetricKind::FractionPositive => h.update([2]),
        MetricKind::ConcentrationIndex => h.update([3]),
        MetricKind::CapacityFraction => h.update([4]),
        MetricKind::Other(id) => {
            h.update([255]);
            put_id(h, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BoundedUncertainty, EvidenceProvenance, GeographicScope, IndependenceGroup,
        ObservationWindow, ObservedMetric, SourceRecordDigest,
    };

    fn observation(producer: &str, digest_algorithm: Digest32Algorithm) -> SurveillanceObservation {
        SurveillanceObservation::new(
            SignalFamily::Respiratory,
            SourceKind::WastewaterAggregate,
            "ww-feed-17",
            IndependenceGroup::new("ww-lab-lineage-17").unwrap(),
            GeographicScope::new("health-district", "district-17", GeographicPrecision::District)
                .unwrap(),
            ObservationWindow::new(10_000, 20_000).unwrap(),
            21_000,
            500,
            ObservedMetric::new(
                MetricKind::ConcentrationIndex,
                1.25,
                BoundedUncertainty::new(1.0, 1.5).unwrap(),
                "normalized_concentration",
            )
            .unwrap(),
            EvidenceProvenance::new(
                producer,
                "ww-protocol-v1",
                "rev-1",
                Some("sampler-aggregate-17"),
                SourceRecordDigest::new(digest_algorithm, [3; 32]).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn identical_semantics_have_identical_identity() {
        let a = observation("lab-a", Digest32Algorithm::Sha256);
        let b = observation("lab-a", Digest32Algorithm::Sha256);
        assert_eq!(a.id().unwrap(), b.id().unwrap());
    }

    #[test]
    fn provenance_changes_identity() {
        let a = observation("lab-a", Digest32Algorithm::Sha256);
        let b = observation("lab-b", Digest32Algorithm::Sha256);
        assert_ne!(a.id().unwrap(), b.id().unwrap());
    }

    #[test]
    fn source_digest_algorithm_changes_identity_even_when_digest_bytes_match() {
        let sha = observation("lab-a", Digest32Algorithm::Sha256);
        let blake3 = observation("lab-a", Digest32Algorithm::Blake3);
        assert_ne!(sha.id().unwrap(), blake3.id().unwrap());
    }

    #[test]
    fn digest_has_stable_hex_shape() {
        let id = observation("lab-a", Digest32Algorithm::Sha256)
            .id()
            .unwrap();
        assert_eq!(id.to_hex().len(), 64);
        assert!(id.to_hex().bytes().all(|b| b.is_ascii_hexdigit()));
    }
}
