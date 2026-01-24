/**
 * Telehealth Zome Client
 *
 * Client for virtual visit scheduling, session management, and telehealth
 * coordination in Mycelix-Health.
 */

import type { AppClient, ActionHash, AgentPubKey, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Session Type
export type SessionType = 'VideoVisit' | 'PhoneCall' | 'SecureMessage' | 'RemoteMonitoring';

// Session Status
export type SessionStatus =
  | 'Scheduled'
  | 'WaitingRoom'
  | 'InProgress'
  | 'Completed'
  | 'Cancelled'
  | 'NoShow'
  | 'Rescheduled';

// Waiting Room Status
export type WaitingRoomStatus = 'Waiting' | 'Called' | 'InSession' | 'Left';

// Telehealth Session
export interface TelehealthSession {
  hash?: ActionHash;
  session_id: string;
  patient_hash: ActionHash;
  provider_hash: ActionHash;
  scheduled_start: Timestamp;
  scheduled_duration_minutes: number;
  session_type: SessionType;
  status: SessionStatus;
  meeting_url?: string;
  meeting_id?: string;
  meeting_password?: string;
  reason_for_visit?: string;
  notes?: string;
  actual_start?: Timestamp;
  actual_end?: Timestamp;
  created_at: Timestamp;
  updated_at: Timestamp;
}

// Waiting Room Entry
export interface WaitingRoomEntry {
  hash?: ActionHash;
  session_hash: ActionHash;
  patient_hash: ActionHash;
  joined_at: Timestamp;
  status: WaitingRoomStatus;
  position_in_queue: number;
  estimated_wait_minutes?: number;
  called_at?: Timestamp;
  notes?: string;
}

// Session Documentation
export interface SessionDocumentation {
  hash?: ActionHash;
  session_hash: ActionHash;
  chief_complaint: string;
  history_of_present_illness?: string;
  assessment: string;
  plan: string;
  follow_up_instructions?: string;
  prescriptions_issued: string[];
  referrals_made: string[];
  documented_by: AgentPubKey;
  documented_at: Timestamp;
}

// Available Slot
export interface AvailableSlot {
  provider_hash: ActionHash;
  start_time: Timestamp;
  end_time: Timestamp;
  session_types: SessionType[];
  is_available: boolean;
}

// Provider Schedule
export interface ProviderSchedule {
  provider_hash: ActionHash;
  date: string; // YYYY-MM-DD
  available_slots: AvailableSlot[];
  blocked_times: Array<{ start: Timestamp; end: Timestamp; reason?: string }>;
}

// Session Summary
export interface SessionSummary {
  session: TelehealthSession;
  documentation?: SessionDocumentation;
  duration_minutes?: number;
  patient_name?: string;
  provider_name?: string;
}

// Input types
export interface ScheduleSessionInput {
  patient_hash: ActionHash;
  provider_hash: ActionHash;
  scheduled_start: Timestamp;
  duration_minutes?: number;
  session_type: SessionType;
  reason_for_visit?: string;
}

export interface CreateDocumentationInput {
  session_hash: ActionHash;
  chief_complaint: string;
  history_of_present_illness?: string;
  assessment: string;
  plan: string;
  follow_up_instructions?: string;
  prescriptions_issued?: string[];
  referrals_made?: string[];
}

export interface GetAvailableSlotsInput {
  provider_hash: ActionHash;
  start_date: string; // YYYY-MM-DD
  end_date: string; // YYYY-MM-DD
  session_type?: SessionType;
}

export interface UpdateSessionInput {
  session_hash: ActionHash;
  status?: SessionStatus;
  meeting_url?: string;
  meeting_id?: string;
  meeting_password?: string;
  notes?: string;
}

/**
 * Telehealth Zome Client
 */
export class TelehealthClient {
  private readonly roleName: string;
  private readonly zomeName = 'telehealth';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Schedule a telehealth session
   */
  async scheduleSession(input: ScheduleSessionInput): Promise<ActionHash> {
    return this.call<ActionHash>('schedule_telehealth_session', {
      ...input,
      duration_minutes: input.duration_minutes ?? 30,
    });
  }

  /**
   * Get a telehealth session
   */
  async getSession(sessionHash: ActionHash): Promise<TelehealthSession | null> {
    return this.call<TelehealthSession | null>('get_session', sessionHash);
  }

  /**
   * Get sessions for a patient
   */
  async getPatientSessions(
    patientHash: ActionHash,
    includeCompleted = false
  ): Promise<TelehealthSession[]> {
    return this.call<TelehealthSession[]>('get_patient_sessions', {
      patient_hash: patientHash,
      include_completed: includeCompleted,
    });
  }

  /**
   * Get sessions for a provider
   */
  async getProviderSessions(
    providerHash: ActionHash,
    date?: string
  ): Promise<TelehealthSession[]> {
    return this.call<TelehealthSession[]>('get_provider_sessions', {
      provider_hash: providerHash,
      date,
    });
  }

  /**
   * Start a session (provider initiates)
   */
  async startSession(sessionHash: ActionHash): Promise<TelehealthSession> {
    return this.call<TelehealthSession>('start_session', sessionHash);
  }

  /**
   * End a session
   */
  async endSession(sessionHash: ActionHash): Promise<TelehealthSession> {
    return this.call<TelehealthSession>('end_session', sessionHash);
  }

  /**
   * Cancel a session
   */
  async cancelSession(sessionHash: ActionHash, reason?: string): Promise<void> {
    await this.call<void>('cancel_session', {
      session_hash: sessionHash,
      reason,
    });
  }

  /**
   * Update a session
   */
  async updateSession(input: UpdateSessionInput): Promise<ActionHash> {
    return this.call<ActionHash>('update_session', input);
  }

  /**
   * Join the waiting room for a session
   */
  async joinWaitingRoom(sessionHash: ActionHash): Promise<WaitingRoomEntry> {
    return this.call<WaitingRoomEntry>('join_waiting_room', sessionHash);
  }

  /**
   * Get the provider's waiting room
   */
  async getWaitingRoom(providerHash: ActionHash): Promise<WaitingRoomEntry[]> {
    return this.call<WaitingRoomEntry[]>('get_waiting_room', providerHash);
  }

  /**
   * Call the next patient from waiting room
   */
  async callNextPatient(providerHash: ActionHash): Promise<WaitingRoomEntry | null> {
    return this.call<WaitingRoomEntry | null>('call_next_patient', providerHash);
  }

  /**
   * Call a specific patient from waiting room
   */
  async callPatient(waitingRoomEntryHash: ActionHash): Promise<WaitingRoomEntry> {
    return this.call<WaitingRoomEntry>('call_patient', waitingRoomEntryHash);
  }

  /**
   * Leave the waiting room
   */
  async leaveWaitingRoom(waitingRoomEntryHash: ActionHash): Promise<void> {
    await this.call<void>('leave_waiting_room', waitingRoomEntryHash);
  }

  /**
   * Create session documentation
   */
  async createDocumentation(input: CreateDocumentationInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_session_documentation', {
      ...input,
      prescriptions_issued: input.prescriptions_issued ?? [],
      referrals_made: input.referrals_made ?? [],
    });
  }

  /**
   * Get session documentation
   */
  async getSessionDocumentation(sessionHash: ActionHash): Promise<SessionDocumentation | null> {
    return this.call<SessionDocumentation | null>('get_session_documentation', sessionHash);
  }

  /**
   * Get available slots for a provider
   */
  async getAvailableSlots(input: GetAvailableSlotsInput): Promise<AvailableSlot[]> {
    return this.call<AvailableSlot[]>('get_available_slots', input);
  }

  /**
   * Get provider's schedule for a date
   */
  async getProviderSchedule(providerHash: ActionHash, date: string): Promise<ProviderSchedule> {
    return this.call<ProviderSchedule>('get_provider_schedule', {
      provider_hash: providerHash,
      date,
    });
  }

  /**
   * Set provider availability
   */
  async setAvailability(
    providerHash: ActionHash,
    slots: Array<{ start: Timestamp; end: Timestamp; session_types: SessionType[] }>
  ): Promise<void> {
    await this.call<void>('set_availability', {
      provider_hash: providerHash,
      slots,
    });
  }

  /**
   * Block time on provider's schedule
   */
  async blockTime(
    providerHash: ActionHash,
    start: Timestamp,
    end: Timestamp,
    reason?: string
  ): Promise<void> {
    await this.call<void>('block_time', {
      provider_hash: providerHash,
      start,
      end,
      reason,
    });
  }

  /**
   * Get upcoming sessions for the current agent
   */
  async getMyUpcomingSessions(): Promise<TelehealthSession[]> {
    return this.call<TelehealthSession[]>('get_my_upcoming_sessions', null);
  }

  /**
   * Get session summary with documentation
   */
  async getSessionSummary(sessionHash: ActionHash): Promise<SessionSummary> {
    return this.call<SessionSummary>('get_session_summary', sessionHash);
  }

  /**
   * Reschedule a session
   */
  async rescheduleSession(
    sessionHash: ActionHash,
    newStart: Timestamp,
    newDuration?: number
  ): Promise<ActionHash> {
    return this.call<ActionHash>('reschedule_session', {
      session_hash: sessionHash,
      new_start: newStart,
      new_duration_minutes: newDuration,
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
        `Telehealth zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
