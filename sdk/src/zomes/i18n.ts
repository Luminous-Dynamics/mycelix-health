/**
 * Internationalization (i18n) Zome Client
 *
 * Client for medical terminology translation and localization.
 * Part of Phase 6 - Global Scale.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Types
export interface LocalizedTerm {
  term_hash: ActionHash;
  terminology: string;
  code: string;
  locale: string;
  display_text: string;
  definition?: string;
  synonyms: string[];
  verified: boolean;
  verified_by?: ActionHash;
}

export interface TranslationMemory {
  memory_hash: ActionHash;
  source_text: string;
  source_locale: string;
  target_text: string;
  target_locale: string;
  domain: string;
  quality_score: number;
  usage_count: number;
  created_at: Timestamp;
}

export interface GlossaryEntry {
  entry_hash: ActionHash;
  source_term: string;
  source_locale: string;
  target_term: string;
  target_locale: string;
  domain: string;
  definition?: string;
  usage_notes?: string;
  do_not_translate: boolean;
}

export interface SupportedLocale {
  code: string;
  name: string;
  native_name: string;
  direction: 'ltr' | 'rtl';
  medical_terminology_coverage: number;
}

// Input types
export interface GetMedicalTermInput {
  terminology: string;
  code: string;
  locale: string;
}

export interface AddLocalizedTermInput {
  terminology: string;
  code: string;
  locale: string;
  display_text: string;
  definition?: string;
  synonyms: string[];
}

export interface TranslateInput {
  text: string;
  source_locale: string;
  target_locale: string;
  domain?: string;
}

export interface AddTranslationMemoryInput {
  source_text: string;
  source_locale: string;
  target_text: string;
  target_locale: string;
  domain: string;
  quality_score: number;
}

export interface AddGlossaryEntryInput {
  source_term: string;
  source_locale: string;
  target_term: string;
  target_locale: string;
  domain: string;
  definition?: string;
  usage_notes?: string;
  do_not_translate: boolean;
}

/**
 * i18n Zome Client
 */
export class I18nClient {
  private readonly roleName: string;
  private readonly zomeName = 'i18n';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Get a localized medical term
   */
  async getMedicalTerm(input: GetMedicalTermInput): Promise<LocalizedTerm | null> {
    return this.call<LocalizedTerm | null>('get_medical_term', input);
  }

  /**
   * Add a localized term
   */
  async addLocalizedTerm(input: AddLocalizedTermInput): Promise<ActionHash> {
    return this.call<ActionHash>('add_localized_term', input);
  }

  /**
   * Verify a translation
   */
  async verifyTranslation(termHash: ActionHash): Promise<void> {
    return this.call<void>('verify_translation', termHash);
  }

  /**
   * Translate text using translation memory
   */
  async translate(input: TranslateInput): Promise<{ translation: string; confidence: number; source: string }> {
    return this.call<{ translation: string; confidence: number; source: string }>('translate', input);
  }

  /**
   * Add to translation memory
   */
  async addTranslationMemory(input: AddTranslationMemoryInput): Promise<ActionHash> {
    return this.call<ActionHash>('add_translation_memory', input);
  }

  /**
   * Get translation suggestions
   */
  async getSuggestions(
    text: string,
    sourceLocale: string,
    targetLocale: string
  ): Promise<TranslationMemory[]> {
    return this.call<TranslationMemory[]>('get_suggestions', {
      text,
      source_locale: sourceLocale,
      target_locale: targetLocale,
    });
  }

  /**
   * Add glossary entry
   */
  async addGlossaryEntry(input: AddGlossaryEntryInput): Promise<ActionHash> {
    return this.call<ActionHash>('add_glossary_entry', input);
  }

  /**
   * Get glossary entries for a domain
   */
  async getGlossaryEntries(input: { domain: string }): Promise<GlossaryEntry[]> {
    return this.call<GlossaryEntry[]>('get_glossary_entries', input);
  }

  /**
   * Get supported locales
   */
  async getSupportedLocales(): Promise<SupportedLocale[]> {
    return this.call<SupportedLocale[]>('get_supported_locales', null);
  }

  /**
   * Get terminology coverage for a locale
   */
  async getTerminologyCoverage(
    terminology: string,
    locale: string
  ): Promise<{ total_terms: number; translated_terms: number; verified_terms: number }> {
    return this.call<{ total_terms: number; translated_terms: number; verified_terms: number }>(
      'get_terminology_coverage',
      { terminology, locale }
    );
  }

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    try {
      const result = await this.client.callZome({
        role_name: this.roleName,
        zome_name: this.zomeName,
        fn_name: fnName,
        payload,
      });
      return result as T;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new HealthSdkError(
        HealthSdkErrorCode.ZOME_CALL_FAILED,
        `i18n zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
