//! Internationalization (i18n) Integrity Zome
//!
//! Defines entry types for multi-language support including
//! translation management, locale preferences, and content localization.

use hdi::prelude::*;

/// Content type for translation
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContentType {
    /// User interface text
    UiLabel,
    /// Medical terminology
    MedicalTerm,
    /// Clinical description
    ClinicalDescription,
    /// Patient education material
    PatientEducation,
    /// Consent form text
    ConsentForm,
    /// Error message
    ErrorMessage,
    /// Notification text
    Notification,
    /// Form field label
    FormField,
    /// Help text
    HelpText,
    /// Legal/regulatory text
    LegalText,
}

/// Translation status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TranslationStatus {
    /// Machine translated, not reviewed
    MachineTranslated,
    /// Human translated
    HumanTranslated,
    /// Reviewed and approved
    Approved,
    /// Needs update (source changed)
    NeedsUpdate,
    /// Rejected
    Rejected,
}

/// Text direction
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

/// Medical terminology standard
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TerminologyStandard {
    /// SNOMED CT
    SnomedCt,
    /// ICD-10
    Icd10,
    /// LOINC
    Loinc,
    /// RxNorm
    RxNorm,
    /// CPT
    Cpt,
    /// MedDRA
    MedDra,
    /// Custom
    Custom,
}

/// Supported locale configuration
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SupportedLocale {
    /// Locale ID
    pub locale_id: String,
    /// Language code (ISO 639-1)
    pub language_code: String,
    /// Country code (ISO 3166-1 alpha-2)
    pub country_code: Option<String>,
    /// Script (ISO 15924)
    pub script: Option<String>,
    /// Full locale tag (BCP 47)
    pub locale_tag: String,
    /// Display name (in this locale)
    pub native_name: String,
    /// Display name in English
    pub english_name: String,
    /// Text direction
    pub text_direction: TextDirection,
    /// Date format pattern
    pub date_format: String,
    /// Time format pattern
    pub time_format: String,
    /// Number format locale
    pub number_format: String,
    /// Currency code (ISO 4217)
    pub currency_code: Option<String>,
    /// Whether this locale is active
    pub is_active: bool,
    /// Translation completeness percentage
    pub completeness_percent: u32,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Translation string entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TranslationString {
    /// Translation key (unique identifier)
    pub translation_key: String,
    /// Locale tag
    pub locale_tag: String,
    /// Content type
    pub content_type: ContentType,
    /// Namespace/category
    pub namespace: String,
    /// Translated text
    pub translated_text: String,
    /// Plural forms (JSON map)
    pub plural_forms: Option<String>,
    /// Context hint for translators
    pub context: Option<String>,
    /// Maximum length constraint
    pub max_length: Option<u32>,
    /// Translation status
    pub status: TranslationStatus,
    /// Translator hash (if human)
    pub translator_hash: Option<ActionHash>,
    /// Reviewer hash (if approved)
    pub reviewer_hash: Option<ActionHash>,
    /// Source text version this translates
    pub source_version: u32,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Source string (base language)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SourceString {
    /// Translation key
    pub translation_key: String,
    /// Namespace/category
    pub namespace: String,
    /// Content type
    pub content_type: ContentType,
    /// Source text (typically English)
    pub source_text: String,
    /// Plural forms (JSON)
    pub plural_forms: Option<String>,
    /// Context for translators
    pub context: Option<String>,
    /// Maximum length
    pub max_length: Option<u32>,
    /// Screenshot URL for context
    pub screenshot_url: Option<String>,
    /// Current version
    pub version: u32,
    /// Whether active
    pub is_active: bool,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Medical term translation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MedicalTermTranslation {
    /// Term ID
    pub term_id: String,
    /// Terminology standard
    pub terminology: TerminologyStandard,
    /// Standard code
    pub code: String,
    /// Locale tag
    pub locale_tag: String,
    /// Preferred term in this locale
    pub preferred_term: String,
    /// Synonyms in this locale
    pub synonyms: Vec<String>,
    /// Consumer-friendly term
    pub consumer_term: Option<String>,
    /// Definition in this locale
    pub definition: Option<String>,
    /// Status
    pub status: TranslationStatus,
    /// Official source (if any)
    pub official_source: Option<String>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// User locale preferences
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct UserLocalePreference {
    /// User/patient hash
    pub user_hash: ActionHash,
    /// Primary locale
    pub primary_locale: String,
    /// Secondary locales (fallback)
    pub secondary_locales: Vec<String>,
    /// Date format preference
    pub date_format_preference: Option<String>,
    /// Time format (12h/24h)
    pub time_format_24h: bool,
    /// First day of week (0=Sun, 1=Mon)
    pub first_day_of_week: u32,
    /// Measurement system (metric/imperial)
    pub measurement_system: String,
    /// Temperature unit (C/F)
    pub temperature_unit: String,
    /// Use consumer-friendly medical terms
    pub use_consumer_terms: bool,
    /// Reading level preference
    pub reading_level: Option<String>,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Translation memory entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TranslationMemory {
    /// Memory entry ID
    pub memory_id: String,
    /// Source text
    pub source_text: String,
    /// Source locale
    pub source_locale: String,
    /// Target text
    pub target_text: String,
    /// Target locale
    pub target_locale: String,
    /// Domain/context
    pub domain: String,
    /// Quality score (0-100)
    pub quality_score: u32,
    /// Usage count
    pub usage_count: u32,
    /// Contributor hash
    pub contributor_hash: Option<ActionHash>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Glossary entry for consistent terminology
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct GlossaryEntry {
    /// Entry ID
    pub entry_id: String,
    /// Term in source language
    pub source_term: String,
    /// Source locale
    pub source_locale: String,
    /// Term in target language
    pub target_term: String,
    /// Target locale
    pub target_locale: String,
    /// Part of speech
    pub part_of_speech: Option<String>,
    /// Definition
    pub definition: Option<String>,
    /// Usage notes for translators
    pub usage_notes: Option<String>,
    /// Do not translate (keep original)
    pub do_not_translate: bool,
    /// Case sensitive
    pub case_sensitive: bool,
    /// Domain
    pub domain: String,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Entry types for the i18n zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    SupportedLocale(SupportedLocale),
    TranslationString(TranslationString),
    SourceString(SourceString),
    MedicalTermTranslation(MedicalTermTranslation),
    UserLocalePreference(UserLocalePreference),
    TranslationMemory(TranslationMemory),
    GlossaryEntry(GlossaryEntry),
}

/// Link types for the i18n zome
#[hdk_link_types]
pub enum LinkTypes {
    /// All supported locales
    AllLocales,
    /// Source strings by namespace
    SourcesByNamespace,
    /// Translations by locale
    TranslationsByLocale,
    /// Translations by key
    TranslationsByKey,
    /// Medical terms by code
    MedicalTermsByCode,
    /// Medical terms by locale
    MedicalTermsByLocale,
    /// User preferences
    UserToPreferences,
    /// Translation memory by locale pair
    MemoryByLocalePair,
    /// Glossary by domain
    GlossaryByDomain,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::SupportedLocale(locale) => validate_locale(&locale),
                EntryTypes::TranslationString(trans) => validate_translation(&trans),
                EntryTypes::UserLocalePreference(pref) => validate_preference(&pref),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_locale(locale: &SupportedLocale) -> ExternResult<ValidateCallbackResult> {
    if locale.locale_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Locale ID is required".to_string()));
    }
    if locale.language_code.len() != 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "Language code must be ISO 639-1 (2 chars)".to_string(),
        ));
    }
    if let Some(ref country) = locale.country_code {
        if country.len() != 2 {
            return Ok(ValidateCallbackResult::Invalid(
                "Country code must be ISO 3166-1 alpha-2 (2 chars)".to_string(),
            ));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_translation(trans: &TranslationString) -> ExternResult<ValidateCallbackResult> {
    if trans.translation_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Translation key is required".to_string()));
    }
    if trans.locale_tag.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Locale tag is required".to_string()));
    }
    if trans.translated_text.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Translated text is required".to_string()));
    }
    if let Some(max_len) = trans.max_length {
        if trans.translated_text.len() > max_len as usize {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "Translation exceeds max length of {}",
                max_len
            )));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_preference(pref: &UserLocalePreference) -> ExternResult<ValidateCallbackResult> {
    if pref.primary_locale.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Primary locale is required".to_string()));
    }
    if pref.first_day_of_week > 6 {
        return Ok(ValidateCallbackResult::Invalid(
            "First day of week must be 0-6".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
