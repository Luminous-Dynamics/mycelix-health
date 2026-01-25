/**
 * Population Health Zome Client
 *
 * Client for community health surveillance and public health analytics.
 * Part of Phase 5 - Advanced Research.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Enums
export enum SurveillanceType {
  Syndromic = 'Syndromic',
  CaseReporting = 'CaseReporting',
  LabReporting = 'LabReporting',
  VitalStatistics = 'VitalStatistics',
  HealthSurvey = 'HealthSurvey',
}

export enum AlertLevel {
  Normal = 'Normal',
  Watch = 'Watch',
  Warning = 'Warning',
  Alert = 'Alert',
  Emergency = 'Emergency',
}

export enum ReportingFrequency {
  RealTime = 'RealTime',
  Daily = 'Daily',
  Weekly = 'Weekly',
  Monthly = 'Monthly',
}

// Types
export interface SurveillanceIndicator {
  indicator_hash: ActionHash;
  name: string;
  description: string;
  surveillance_type: SurveillanceType;
  baseline_value: number;
  threshold_warning: number;
  threshold_alert: number;
  unit: string;
  reporting_frequency: ReportingFrequency;
}

export interface PopulationMetric {
  metric_hash: ActionHash;
  indicator_hash: ActionHash;
  region: string;
  value: number;
  sample_size: number;
  confidence_interval_low: number;
  confidence_interval_high: number;
  recorded_at: Timestamp;
}

export interface HealthAlert {
  alert_hash: ActionHash;
  indicator_hash: ActionHash;
  region: string;
  level: AlertLevel;
  current_value: number;
  baseline_value: number;
  message: string;
  issued_at: Timestamp;
  expires_at?: Timestamp;
  acknowledged: boolean;
}

export interface PopulationTrend {
  indicator_hash: ActionHash;
  region: string;
  trend_direction: 'increasing' | 'decreasing' | 'stable';
  percent_change: number;
  period_start: Timestamp;
  period_end: Timestamp;
}

export interface CommunityHealthProfile {
  region: string;
  population: number;
  indicators: SurveillanceIndicator[];
  current_alerts: HealthAlert[];
  health_score: number;
  last_updated: Timestamp;
}

// Input types
export interface CreateIndicatorInput {
  name: string;
  description: string;
  surveillance_type: SurveillanceType;
  baseline_value: number;
  threshold_warning: number;
  threshold_alert: number;
  unit: string;
  reporting_frequency: ReportingFrequency;
}

export interface RecordMetricInput {
  indicator_hash: ActionHash;
  region: string;
  value: number;
  sample_size: number;
  confidence_interval_low: number;
  confidence_interval_high: number;
}

export interface GetTrendsInput {
  indicator_hash: ActionHash;
  region: string;
  period_days: number;
}

/**
 * Population Health Zome Client
 */
export class PopulationHealthClient {
  private readonly roleName: string;
  private readonly zomeName = 'population_health';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Create a surveillance indicator
   */
  async createIndicator(input: CreateIndicatorInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_indicator', input);
  }

  /**
   * Get an indicator by hash
   */
  async getIndicator(indicatorHash: ActionHash): Promise<SurveillanceIndicator | null> {
    return this.call<SurveillanceIndicator | null>('get_indicator', indicatorHash);
  }

  /**
   * List all indicators
   */
  async listIndicators(): Promise<SurveillanceIndicator[]> {
    return this.call<SurveillanceIndicator[]>('list_indicators', null);
  }

  /**
   * Record a population metric
   */
  async recordMetric(input: RecordMetricInput): Promise<ActionHash> {
    return this.call<ActionHash>('record_metric', input);
  }

  /**
   * Get metrics for an indicator in a region
   */
  async getMetrics(indicatorHash: ActionHash, region: string): Promise<PopulationMetric[]> {
    return this.call<PopulationMetric[]>('get_metrics', {
      indicator_hash: indicatorHash,
      region,
    });
  }

  /**
   * Get current alerts
   */
  async getCurrentAlerts(region?: string): Promise<HealthAlert[]> {
    return this.call<HealthAlert[]>('get_current_alerts', region ?? null);
  }

  /**
   * Acknowledge an alert
   */
  async acknowledgeAlert(alertHash: ActionHash): Promise<void> {
    return this.call<void>('acknowledge_alert', alertHash);
  }

  /**
   * Get trends for an indicator
   */
  async getTrends(input: GetTrendsInput): Promise<PopulationTrend> {
    return this.call<PopulationTrend>('get_trends', input);
  }

  /**
   * Get community health profile
   */
  async getCommunityProfile(region: string): Promise<CommunityHealthProfile> {
    return this.call<CommunityHealthProfile>('get_community_profile', region);
  }

  /**
   * Compare regions
   */
  async compareRegions(regions: string[], indicatorHash: ActionHash): Promise<PopulationMetric[]> {
    return this.call<PopulationMetric[]>('compare_regions', {
      regions,
      indicator_hash: indicatorHash,
    });
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
        `Population Health zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
