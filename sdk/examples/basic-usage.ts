/**
 * Basic Usage Example for @mycelix/health-sdk
 *
 * This example demonstrates the core SDK functionality:
 * - Connecting to Holochain
 * - Creating patient records
 * - Managing consent
 * - Executing differentially private queries
 *
 * Prerequisites:
 * - Holochain conductor running with mycelix-health hApp installed
 * - npm install @mycelix/health-sdk
 */

import {
  MycelixHealthClient,
  PrivacyBudgetManager,
  HealthSdkError,
  HealthSdkErrorCode,
  RECOMMENDED_EPSILON,
} from '../src';

async function main() {
  console.log('=== Mycelix Health SDK Basic Usage ===\n');

  // 1. Connect to Holochain
  console.log('1. Connecting to Holochain...');
  const health = await MycelixHealthClient.connect({
    url: 'ws://localhost:8888',
    appId: 'mycelix-health',
    debug: true,
  });
  console.log('   Connected! Agent:', Buffer.from(health.getAgentPubKey()).toString('hex').slice(0, 16) + '...');

  // 2. Create a patient record
  console.log('\n2. Creating patient record...');
  const patient = await health.patients.createPatient({
    first_name: 'Jane',
    last_name: 'Doe',
    date_of_birth: '1990-05-15',
    contact: {
      email: 'jane.doe@example.com',
      phone: '555-0123',
      preferred_contact_method: 'Email',
    },
    emergency_contacts: [
      {
        name: 'John Doe',
        relationship: 'Spouse',
        phone: '555-0124',
      },
    ],
    allergies: ['Penicillin', 'Peanuts'],
    medications: ['Metformin 500mg'],
  });
  console.log('   Patient created:', patient.hash);

  // 3. Create a data pool for research
  console.log('\n3. Creating research data pool...');
  const pool = await health.commons.createPool({
    name: 'Diabetes Research Study 2024',
    description: 'Aggregate data for diabetes outcomes research',
    data_categories: ['lab_results', 'medications', 'vitals'],
    required_consent_level: 'ResearchOnly',
    default_epsilon: 1.0,
    budget_per_user: 10.0,
    governance_model: 'Democratic',
    min_contributors: 50,
  });
  console.log('   Pool created:', pool.hash);

  // 4. Grant consent for research
  console.log('\n4. Granting research consent...');
  const consent = await health.consent.grantConsent(patient.hash, {
    grantee: health.getAgentPubKey(), // Self-grant for demo
    scope: 'ResearchOnly',
    data_categories: ['lab_results', 'medications'],
    purpose: 'Participation in Diabetes Research Study 2024',
  });
  console.log('   Consent granted:', consent.hash);

  // 5. Check privacy budget before querying
  console.log('\n5. Checking privacy budget...');
  const budgetStatus = await health.commons.getBudgetStatus(patient.hash, pool.hash);
  const displayInfo = PrivacyBudgetManager.getDisplayInfo(budgetStatus);

  console.log('   Budget Status:');
  console.log(`     - Total: ${displayInfo.total} epsilon`);
  console.log(`     - Remaining: ${displayInfo.remaining} epsilon (${displayInfo.percentRemaining}%)`);
  console.log(`     - Severity: ${displayInfo.severity}`);
  console.log(`     - Can Query: ${displayInfo.canQuery}`);

  // 6. Estimate how many queries we can make
  console.log('\n6. Planning query budget...');
  const queriesAtHighPrivacy = PrivacyBudgetManager.estimateRemainingQueries(
    budgetStatus,
    RECOMMENDED_EPSILON.HIGH_SENSITIVITY
  );
  const queriesAtModeratePrivacy = PrivacyBudgetManager.estimateRemainingQueries(
    budgetStatus,
    RECOMMENDED_EPSILON.MODERATE_SENSITIVITY
  );

  console.log(`   At high privacy (ε=${RECOMMENDED_EPSILON.HIGH_SENSITIVITY}): ${queriesAtHighPrivacy} queries possible`);
  console.log(`   At moderate privacy (ε=${RECOMMENDED_EPSILON.MODERATE_SENSITIVITY}): ${queriesAtModeratePrivacy} queries possible`);

  // 7. Execute a differentially private count query
  console.log('\n7. Executing DP count query...');
  try {
    const countResult = await health.commons.countWithPrivacy(
      pool.hash,
      patient.hash,
      RECOMMENDED_EPSILON.MODERATE_SENSITIVITY
    );

    console.log('   Query Result:');
    console.log(`     - Noisy Count: ${countResult.value}`);
    console.log(`     - Epsilon Consumed: ${countResult.epsilon_consumed}`);
    if (countResult.confidence_interval) {
      console.log(`     - 95% CI: [${countResult.confidence_interval[0]}, ${countResult.confidence_interval[1]}]`);
    }
  } catch (error) {
    if (error instanceof HealthSdkError) {
      console.log(`   Query failed: ${error.code} - ${error.message}`);
    } else {
      throw error;
    }
  }

  // 8. Check updated budget
  console.log('\n8. Checking updated budget...');
  const updatedStatus = await health.commons.getBudgetStatus(patient.hash, pool.hash);
  const updatedDisplay = PrivacyBudgetManager.getDisplayInfo(updatedStatus);

  console.log(`   Budget after query: ${updatedDisplay.remaining} epsilon (${updatedDisplay.percentRemaining}%)`);
  console.log(`   Queries answered: ${updatedStatus.queriesAnswered}`);

  // 9. Simulate future consumption
  console.log('\n9. Simulating future queries...');
  const plannedQueries = [0.5, 0.5, 0.5, 1.0, 1.0];
  const simulated = PrivacyBudgetManager.simulateConsumption(updatedStatus, plannedQueries);

  console.log(`   After ${plannedQueries.length} planned queries:`);
  console.log(`     - Budget remaining: ${simulated.remaining.toFixed(4)} epsilon`);
  console.log(`     - Would be exhausted: ${simulated.isExhausted}`);

  // 10. Check consent summary
  console.log('\n10. Checking consent summary...');
  const consentSummary = await health.consent.getConsentSummary(patient.hash);

  console.log('    Consent Summary:');
  console.log(`      - Active consents: ${consentSummary.activeCount}`);
  console.log(`      - Expired consents: ${consentSummary.expiredCount}`);
  console.log(`      - Revoked consents: ${consentSummary.revokedCount}`);
  console.log(`      - Unique grantees: ${consentSummary.grantees.length}`);

  console.log('\n=== Example Complete ===');
}

// Run the example
main().catch((error) => {
  console.error('Example failed:', error);
  process.exit(1);
});
