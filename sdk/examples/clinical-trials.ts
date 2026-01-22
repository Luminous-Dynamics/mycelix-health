/**
 * Clinical Trials Example for @mycelix/health-sdk
 *
 * This example demonstrates clinical trial workflows:
 * - Creating trials
 * - Checking eligibility
 * - Enrolling patients
 * - Reporting adverse events
 *
 * Prerequisites:
 * - Holochain conductor running with mycelix-health hApp installed
 * - npm install @mycelix/health-sdk
 */

import {
  MycelixHealthClient,
  HealthSdkError,
  HealthSdkErrorCode,
} from '../src';

async function main() {
  console.log('=== Clinical Trials Example ===\n');

  // Connect to Holochain
  console.log('Connecting to Holochain...');
  const health = await MycelixHealthClient.connect({
    url: 'ws://localhost:8888',
    appId: 'mycelix-health',
  });
  console.log('Connected!\n');

  // Create a patient for trial enrollment
  console.log('1. Creating patient record...');
  const patient = await health.patients.createPatient({
    first_name: 'Robert',
    last_name: 'Smith',
    date_of_birth: '1975-08-22', // 48 years old
    contact: { email: 'robert.smith@example.com' },
    allergies: [],
    medications: ['Metformin 500mg', 'Lisinopril 10mg'],
  });
  console.log('   Patient created:', patient.hash);

  // Create a clinical trial
  console.log('\n2. Creating clinical trial...');
  const trial = await health.trials.createTrial({
    trial_id: 'NCT98765432',
    title: 'Phase 3 Study of Novel Diabetes Treatment',
    description: 'A randomized, double-blind study to evaluate efficacy and safety',
    sponsor: 'Acme Pharmaceuticals',
    phase: 'Phase3',
    eligibility_criteria: {
      min_age: 30,
      max_age: 65,
      gender: 'All',
      conditions: ['Type 2 Diabetes'],
      exclusions: ['Kidney Disease Stage 4+', 'Heart Failure', 'Pregnancy'],
      required_tests: ['HbA1c', 'eGFR', 'Liver Function'],
    },
    target_enrollment: 500,
    start_date: BigInt(Date.now() * 1000),
    expected_end_date: BigInt((Date.now() + 365 * 24 * 60 * 60 * 1000) * 1000), // 1 year
    ind_number: 'IND-123456',
    irb_approval: 'IRB-2024-0123',
  });
  console.log('   Trial created:', trial.hash);
  console.log('   Trial ID:', trial.trial.trial_id);
  console.log('   Target enrollment:', trial.trial.target_enrollment);

  // Check eligibility
  console.log('\n3. Checking patient eligibility...');
  const eligibility = await health.trials.checkEligibility(trial.hash, patient.hash);

  console.log('   Eligibility Result:');
  console.log(`     - Eligible: ${eligibility.eligible}`);
  if (!eligibility.eligible) {
    console.log('     - Reasons:', eligibility.reasons.join(', '));
    console.log('     - Unmet criteria:', eligibility.unmetCriteria.join(', '));
  }

  // Grant consent for trial participation
  console.log('\n4. Granting trial consent...');
  const consent = await health.consent.grantConsent(patient.hash, {
    grantee: health.getAgentPubKey(),
    scope: 'FullAccess',
    data_categories: ['trial_data', 'lab_results', 'medications', 'adverse_events'],
    purpose: `Participation in clinical trial ${trial.trial.trial_id}`,
  });
  console.log('   Consent granted:', consent.hash);

  // Enroll patient (if eligible)
  if (eligibility.eligible) {
    console.log('\n5. Enrolling patient in trial...');
    try {
      const enrollment = await health.trials.enrollPatient(
        trial.hash,
        patient.hash,
        consent.hash
      );
      console.log('   Enrollment successful!');
      console.log('   Enrollment hash:', enrollment.hash);
      console.log('   Status:', enrollment.status);
    } catch (error) {
      if (error instanceof HealthSdkError) {
        console.log('   Enrollment failed:', error.message);
      } else {
        throw error;
      }
    }
  } else {
    console.log('\n5. Skipping enrollment (patient not eligible)');
  }

  // List recruiting trials
  console.log('\n6. Listing recruiting trials...');
  const recruitingTrials = await health.trials.listRecruitingTrials();
  console.log(`   Found ${recruitingTrials.length} recruiting trial(s)`);
  for (const t of recruitingTrials) {
    console.log(`     - ${t.trial.title} (${t.trial.trial_id})`);
  }

  // Get trial statistics
  console.log('\n7. Getting trial statistics...');
  const stats = await health.trials.getTrialStatistics(trial.hash);
  console.log('   Trial Statistics:');
  console.log(`     - Total enrolled: ${stats.totalEnrolled}`);
  console.log(`     - Active participants: ${stats.activeParticipants}`);
  console.log(`     - Withdrawn: ${stats.withdrawnCount}`);
  console.log(`     - Completed: ${stats.completedCount}`);
  console.log(`     - Adverse events: ${stats.adverseEventCount}`);
  console.log(`     - Severe adverse events: ${stats.severeAdverseEventCount}`);

  // Report an adverse event (simulated)
  console.log('\n8. Reporting adverse event...');
  const adverseEventHash = await health.trials.reportAdverseEvent({
    trial_hash: trial.hash,
    patient_hash: patient.hash,
    event_type: 'Gastrointestinal',
    severity: 'Mild',
    description: 'Patient reported mild nausea 2 hours after first dose',
    onset_date: BigInt(Date.now() * 1000),
    related_to_treatment: true,
  });
  console.log('   Adverse event reported:', adverseEventHash);

  // List adverse events for the trial
  console.log('\n9. Listing trial adverse events...');
  const adverseEvents = await health.trials.listTrialAdverseEvents(trial.hash);
  console.log(`   Found ${adverseEvents.length} adverse event(s)`);
  for (const ae of adverseEvents) {
    console.log(`     - ${ae.event_type} (${ae.severity}): ${ae.description.slice(0, 50)}...`);
  }

  // Get patient's trial enrollments
  console.log('\n10. Getting patient enrollments...');
  const enrollments = await health.trials.listPatientEnrollments(patient.hash);
  console.log(`   Patient is enrolled in ${enrollments.length} trial(s)`);

  console.log('\n=== Clinical Trials Example Complete ===');
}

// Run the example
main().catch((error) => {
  console.error('Example failed:', error);
  process.exit(1);
});
