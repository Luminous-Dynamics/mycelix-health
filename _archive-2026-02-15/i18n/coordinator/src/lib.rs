//! Internationalization (i18n) Coordinator Zome
//!
//! Provides functions for multi-language support including
//! translation management, locale preferences, and content localization.

use hdk::prelude::*;
use i18n_integrity::*;

/// Input for adding a supported locale
#[derive(Serialize, Deserialize, Debug)]
pub struct AddLocaleInput {
    pub language_code: String,
    pub country_code: Option<String>,
    pub script: Option<String>,
    pub native_name: String,
    pub english_name: String,
    pub text_direction: TextDirection,
    pub date_format: String,
    pub time_format: String,
    pub number_format: String,
    pub currency_code: Option<String>,
}

/// Input for adding a translation
#[derive(Serialize, Deserialize, Debug)]
pub struct AddTranslationInput {
    pub translation_key: String,
    pub locale_tag: String,
    pub content_type: ContentType,
    pub namespace: String,
    pub translated_text: String,
    pub plural_forms: Option<String>,
    pub context: Option<String>,
    pub max_length: Option<u32>,
    pub status: TranslationStatus,
}

/// Input for adding a source string
#[derive(Serialize, Deserialize, Debug)]
pub struct AddSourceInput {
    pub translation_key: String,
    pub namespace: String,
    pub content_type: ContentType,
    pub source_text: String,
    pub plural_forms: Option<String>,
    pub context: Option<String>,
    pub max_length: Option<u32>,
    pub screenshot_url: Option<String>,
}

/// Input for medical term translation
#[derive(Serialize, Deserialize, Debug)]
pub struct AddMedicalTermInput {
    pub terminology: TerminologyStandard,
    pub code: String,
    pub locale_tag: String,
    pub preferred_term: String,
    pub synonyms: Vec<String>,
    pub consumer_term: Option<String>,
    pub definition: Option<String>,
    pub official_source: Option<String>,
}

/// Input for user preferences
#[derive(Serialize, Deserialize, Debug)]
pub struct SetPreferencesInput {
    pub primary_locale: String,
    pub secondary_locales: Vec<String>,
    pub date_format_preference: Option<String>,
    pub time_format_24h: bool,
    pub first_day_of_week: u32,
    pub measurement_system: String,
    pub temperature_unit: String,
    pub use_consumer_terms: bool,
    pub reading_level: Option<String>,
}

/// Input for translation lookup
#[derive(Serialize, Deserialize, Debug)]
pub struct GetTranslationInput {
    pub key: String,
    pub locale: String,
    pub namespace: Option<String>,
}

/// Input for getting medical term
#[derive(Serialize, Deserialize, Debug)]
pub struct GetMedicalTermInput {
    pub terminology: String,
    pub code: String,
    pub locale: String,
}

/// Input for adding to translation memory
#[derive(Serialize, Deserialize, Debug)]
pub struct AddTranslationMemoryInput {
    pub source_text: String,
    pub source_locale: String,
    pub target_text: String,
    pub target_locale: String,
    pub domain: String,
    pub quality_score: u32,
}

/// Input for adding glossary entry
#[derive(Serialize, Deserialize, Debug)]
pub struct AddGlossaryEntryInput {
    pub source_term: String,
    pub source_locale: String,
    pub target_term: String,
    pub target_locale: String,
    pub domain: String,
    pub definition: Option<String>,
    pub usage_notes: Option<String>,
    pub do_not_translate: bool,
}

/// Input for getting glossary entries
#[derive(Serialize, Deserialize, Debug)]
pub struct GetGlossaryEntriesInput {
    pub domain: String,
}

/// Add a new supported locale
#[hdk_extern]
pub fn add_supported_locale(input: AddLocaleInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    // Build locale tag (BCP 47)
    let locale_tag = if let Some(ref country) = input.country_code {
        format!("{}-{}", input.language_code.to_lowercase(), country.to_uppercase())
    } else {
        input.language_code.to_lowercase()
    };

    let locale = SupportedLocale {
        locale_id: locale_tag.clone(),
        language_code: input.language_code.to_lowercase(),
        country_code: input.country_code.map(|c| c.to_uppercase()),
        script: input.script,
        locale_tag: locale_tag.clone(),
        native_name: input.native_name,
        english_name: input.english_name,
        text_direction: input.text_direction,
        date_format: input.date_format,
        time_format: input.time_format,
        number_format: input.number_format,
        currency_code: input.currency_code,
        is_active: true,
        completeness_percent: 0,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::SupportedLocale(locale))?;

    // Link to all locales
    let all_anchor = anchor_hash("all_locales")?;
    create_link(all_anchor, action_hash.clone(), LinkTypes::AllLocales, ())?;

    Ok(action_hash)
}

/// Get all supported locales
#[hdk_extern]
pub fn get_supported_locales(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("all_locales")?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::AllLocales)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Add a source string (base language)
#[hdk_extern]
pub fn add_source_string(input: AddSourceInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let source = SourceString {
        translation_key: input.translation_key.clone(),
        namespace: input.namespace.clone(),
        content_type: input.content_type,
        source_text: input.source_text,
        plural_forms: input.plural_forms,
        context: input.context,
        max_length: input.max_length,
        screenshot_url: input.screenshot_url,
        version: 1,
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::SourceString(source))?;

    // Link by namespace
    let ns_anchor = anchor_hash(&format!("sources_{}", input.namespace))?;
    create_link(
        ns_anchor,
        action_hash.clone(),
        LinkTypes::SourcesByNamespace,
        (),
    )?;

    Ok(action_hash)
}

/// Get source strings by namespace
#[hdk_extern]
pub fn get_source_strings(namespace: String) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("sources_{}", namespace))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::SourcesByNamespace)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Add a translation string
#[hdk_extern]
pub fn add_translation(input: AddTranslationInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let translator_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let translation = TranslationString {
        translation_key: input.translation_key.clone(),
        locale_tag: input.locale_tag.clone(),
        content_type: input.content_type,
        namespace: input.namespace,
        translated_text: input.translated_text,
        plural_forms: input.plural_forms,
        context: input.context,
        max_length: input.max_length,
        status: input.status.clone(),
        translator_hash: if input.status == TranslationStatus::HumanTranslated {
            Some(translator_hash)
        } else {
            None
        },
        reviewer_hash: None,
        source_version: 1,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::TranslationString(translation))?;

    // Link by locale
    let locale_anchor = anchor_hash(&format!("translations_{}", input.locale_tag))?;
    create_link(
        locale_anchor,
        action_hash.clone(),
        LinkTypes::TranslationsByLocale,
        (),
    )?;

    // Link by key
    let key_anchor = anchor_hash(&format!("key_{}", input.translation_key))?;
    create_link(
        key_anchor,
        action_hash.clone(),
        LinkTypes::TranslationsByKey,
        (),
    )?;

    Ok(action_hash)
}

/// Get translations for a locale
#[hdk_extern]
pub fn get_translations_for_locale(locale_tag: String) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("translations_{}", locale_tag))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::TranslationsByLocale)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Get a specific translation
#[hdk_extern]
pub fn get_translation(input: GetTranslationInput) -> ExternResult<Option<Record>> {
    let key_anchor = anchor_hash(&format!("key_{}", input.key))?;
    let links = get_links(LinkQuery::try_new(key_anchor, LinkTypes::TranslationsByKey)?, GetStrategy::default())?;

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                let trans: TranslationString = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid translation".to_string())))?;

                if trans.locale_tag == input.locale {
                    if let Some(ref ns) = input.namespace {
                        if &trans.namespace == ns {
                            return Ok(Some(record));
                        }
                    } else {
                        return Ok(Some(record));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Add a medical term translation
#[hdk_extern]
pub fn add_medical_term(input: AddMedicalTermInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let term = MedicalTermTranslation {
        term_id: format!("{}-{}-{}", input.terminology.clone() as i32, input.code, input.locale_tag),
        terminology: input.terminology.clone(),
        code: input.code.clone(),
        locale_tag: input.locale_tag.clone(),
        preferred_term: input.preferred_term,
        synonyms: input.synonyms,
        consumer_term: input.consumer_term,
        definition: input.definition,
        status: TranslationStatus::Approved,
        official_source: input.official_source,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::MedicalTermTranslation(term))?;

    // Link by code
    let code_anchor = anchor_hash(&format!("medterm_{}_{}", input.terminology.clone() as i32, input.code))?;
    create_link(
        code_anchor,
        action_hash.clone(),
        LinkTypes::MedicalTermsByCode,
        (),
    )?;

    // Link by locale
    let locale_anchor = anchor_hash(&format!("medterms_{}", input.locale_tag))?;
    create_link(
        locale_anchor,
        action_hash.clone(),
        LinkTypes::MedicalTermsByLocale,
        (),
    )?;

    Ok(action_hash)
}

/// Get medical term by code and locale
#[hdk_extern]
pub fn get_medical_term(input: GetMedicalTermInput) -> ExternResult<Option<Record>> {
    let anchor = anchor_hash(&format!("medterm_{}_{}", input.terminology, input.code))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::MedicalTermsByCode)?, GetStrategy::default())?;

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                let term: MedicalTermTranslation = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid term".to_string())))?;

                if term.locale_tag == input.locale {
                    return Ok(Some(record));
                }
            }
        }
    }

    Ok(None)
}

/// Set user locale preferences
#[hdk_extern]
pub fn set_user_preferences(input: SetPreferencesInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let user_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let preference = UserLocalePreference {
        user_hash: user_hash.clone(),
        primary_locale: input.primary_locale,
        secondary_locales: input.secondary_locales,
        date_format_preference: input.date_format_preference,
        time_format_24h: input.time_format_24h,
        first_day_of_week: input.first_day_of_week,
        measurement_system: input.measurement_system,
        temperature_unit: input.temperature_unit,
        use_consumer_terms: input.use_consumer_terms,
        reading_level: input.reading_level,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::UserLocalePreference(preference))?;

    // Link from user
    create_link(
        user_hash,
        action_hash.clone(),
        LinkTypes::UserToPreferences,
        (),
    )?;

    Ok(action_hash)
}

/// Get user's locale preferences
#[hdk_extern]
pub fn get_my_preferences(_: ()) -> ExternResult<Option<Record>> {
    let agent = agent_info()?.agent_initial_pubkey;
    let user_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let links = get_links(LinkQuery::try_new(user_hash, LinkTypes::UserToPreferences)?, GetStrategy::default())?;

    // Return most recent preference
    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}

/// Add entry to translation memory
#[hdk_extern]
pub fn add_to_translation_memory(input: AddTranslationMemoryInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let contributor_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let memory = TranslationMemory {
        memory_id: format!("tm-{}", now.as_micros()),
        source_text: input.source_text,
        source_locale: input.source_locale.clone(),
        target_text: input.target_text,
        target_locale: input.target_locale.clone(),
        domain: input.domain,
        quality_score: input.quality_score,
        usage_count: 1,
        contributor_hash: Some(contributor_hash),
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::TranslationMemory(memory))?;

    // Link by locale pair
    let pair_anchor = anchor_hash(&format!("tm_{}_to_{}", input.source_locale, input.target_locale))?;
    create_link(
        pair_anchor,
        action_hash.clone(),
        LinkTypes::MemoryByLocalePair,
        (),
    )?;

    Ok(action_hash)
}

/// Add glossary entry
#[hdk_extern]
pub fn add_glossary_entry(input: AddGlossaryEntryInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let entry = GlossaryEntry {
        entry_id: format!("gloss-{}", now.as_micros()),
        source_term: input.source_term,
        source_locale: input.source_locale,
        target_term: input.target_term,
        target_locale: input.target_locale,
        part_of_speech: None,
        definition: input.definition,
        usage_notes: input.usage_notes,
        do_not_translate: input.do_not_translate,
        case_sensitive: false,
        domain: input.domain.clone(),
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::GlossaryEntry(entry))?;

    // Link by domain
    let domain_anchor = anchor_hash(&format!("glossary_{}", input.domain))?;
    create_link(
        domain_anchor,
        action_hash.clone(),
        LinkTypes::GlossaryByDomain,
        (),
    )?;

    Ok(action_hash)
}

/// Get glossary entries for domain
#[hdk_extern]
pub fn get_glossary_entries(input: GetGlossaryEntriesInput) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash(&format!("glossary_{}", input.domain))?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::GlossaryByDomain)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

// Helper function
/// Anchor for linking entries
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor: &str) -> ExternResult<AnyLinkableHash> {
    let anchor = Anchor(anchor.to_string());
    Ok(hash_entry(&anchor)?.into())
}
