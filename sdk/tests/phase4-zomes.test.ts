import { describe, it, expect } from 'vitest';
import {
  // SDOH types
  ScreeningInstrument,
  SdohDomain,
  SdohCategory,
  RiskLevel,
  InterventionStatus,
  ResourceType,

  // Mental Health types
  MentalHealthInstrument,
  Severity,
  CrisisLevel,
  TreatmentModality,
  SafetyPlanStatus,
  SubstanceCategory,
  Part2ConsentType,

  // Chronic Care types
  DiabetesType,
  NYHAClass,
  GOLDStage,
  CKDStage,
  AlertSeverity,

  // Pediatric types
  VaccineType,
  ImmunizationStatus,
  DevelopmentalDomain,
  MilestoneStatus,
  FeedingType,
} from '../src/zomes';

describe('SDOH Types', () => {
  describe('ScreeningInstrument', () => {
    it('should have standard SDOH screening instruments', () => {
      expect(ScreeningInstrument.PRAPARE).toBe('PRAPARE');
      expect(ScreeningInstrument.AHCHRSN).toBe('AHCHRSN');
      expect(ScreeningInstrument.WeCare).toBe('WeCare');
      expect(ScreeningInstrument.Custom).toBe('Custom');
    });
  });

  describe('SdohDomain', () => {
    it('should have all SDOH domains', () => {
      expect(SdohDomain.EconomicStability).toBe('EconomicStability');
      expect(SdohDomain.EducationAccess).toBe('EducationAccess');
      expect(SdohDomain.HealthcareAccess).toBe('HealthcareAccess');
      expect(SdohDomain.NeighborhoodEnvironment).toBe('NeighborhoodEnvironment');
      expect(SdohDomain.SocialCommunity).toBe('SocialCommunity');
    });
  });

  describe('SdohCategory', () => {
    it('should have SDOH categories', () => {
      expect(SdohCategory.FoodInsecurity).toBe('FoodInsecurity');
      expect(SdohCategory.HousingInstability).toBe('HousingInstability');
      expect(SdohCategory.Transportation).toBe('Transportation');
      expect(SdohCategory.Utilities).toBe('Utilities');
      expect(SdohCategory.InterpersonalViolence).toBe('InterpersonalViolence');
      expect(SdohCategory.Employment).toBe('Employment');
      expect(SdohCategory.SocialIsolation).toBe('SocialIsolation');
    });
  });

  describe('RiskLevel', () => {
    it('should have all risk levels', () => {
      expect(RiskLevel.NoRisk).toBe('NoRisk');
      expect(RiskLevel.LowRisk).toBe('LowRisk');
      expect(RiskLevel.ModerateRisk).toBe('ModerateRisk');
      expect(RiskLevel.HighRisk).toBe('HighRisk');
      expect(RiskLevel.Urgent).toBe('Urgent');
    });
  });

  describe('InterventionStatus', () => {
    it('should have intervention statuses', () => {
      expect(InterventionStatus.Identified).toBe('Identified');
      expect(InterventionStatus.ReferralMade).toBe('ReferralMade');
      expect(InterventionStatus.InProgress).toBe('InProgress');
      expect(InterventionStatus.Completed).toBe('Completed');
      expect(InterventionStatus.Declined).toBe('Declined');
      expect(InterventionStatus.UnableToComplete).toBe('UnableToComplete');
    });
  });

  describe('ResourceType', () => {
    it('should have resource types', () => {
      expect(ResourceType.FoodPantry).toBe('FoodPantry');
      expect(ResourceType.HousingAssistance).toBe('HousingAssistance');
      expect(ResourceType.TransportationService).toBe('TransportationService');
      expect(ResourceType.UtilityAssistance).toBe('UtilityAssistance');
      expect(ResourceType.EmploymentServices).toBe('EmploymentServices');
      expect(ResourceType.EducationProgram).toBe('EducationProgram');
      expect(ResourceType.LegalAid).toBe('LegalAid');
      expect(ResourceType.ChildcareServices).toBe('ChildcareServices');
      expect(ResourceType.DomesticViolenceServices).toBe('DomesticViolenceServices');
      expect(ResourceType.MentalHealthServices).toBe('MentalHealthServices');
      expect(ResourceType.SubstanceAbuseServices).toBe('SubstanceAbuseServices');
      expect(ResourceType.Other).toBe('Other');
    });
  });
});

describe('Mental Health Types', () => {
  describe('MentalHealthInstrument', () => {
    it('should have all screening instruments', () => {
      expect(MentalHealthInstrument.PHQ9).toBe('PHQ9');
      expect(MentalHealthInstrument.PHQ2).toBe('PHQ2');
      expect(MentalHealthInstrument.GAD7).toBe('GAD7');
      expect(MentalHealthInstrument.CSSRS).toBe('CSSRS');
      expect(MentalHealthInstrument.CAGE).toBe('CAGE');
      expect(MentalHealthInstrument.AUDIT).toBe('AUDIT');
      expect(MentalHealthInstrument.DAST10).toBe('DAST10');
      expect(MentalHealthInstrument.PCL5).toBe('PCL5');
      expect(MentalHealthInstrument.MDQ).toBe('MDQ');
      expect(MentalHealthInstrument.EPDS).toBe('EPDS');
      expect(MentalHealthInstrument.PSC17).toBe('PSC17');
      expect(MentalHealthInstrument.Custom).toBe('Custom');
    });
  });

  describe('Severity', () => {
    it('should have all severity levels', () => {
      expect(Severity.None).toBe('None');
      expect(Severity.Minimal).toBe('Minimal');
      expect(Severity.Mild).toBe('Mild');
      expect(Severity.Moderate).toBe('Moderate');
      expect(Severity.ModeratelySevere).toBe('ModeratelySevere');
      expect(Severity.Severe).toBe('Severe');
    });
  });

  describe('CrisisLevel', () => {
    it('should have all crisis levels', () => {
      expect(CrisisLevel.None).toBe('None');
      expect(CrisisLevel.LowRisk).toBe('LowRisk');
      expect(CrisisLevel.ModerateRisk).toBe('ModerateRisk');
      expect(CrisisLevel.HighRisk).toBe('HighRisk');
      expect(CrisisLevel.Imminent).toBe('Imminent');
    });
  });

  describe('TreatmentModality', () => {
    it('should have all treatment modalities', () => {
      expect(TreatmentModality.IndividualTherapy).toBe('IndividualTherapy');
      expect(TreatmentModality.GroupTherapy).toBe('GroupTherapy');
      expect(TreatmentModality.FamilyTherapy).toBe('FamilyTherapy');
      expect(TreatmentModality.Medication).toBe('Medication');
      expect(TreatmentModality.IntensiveOutpatient).toBe('IntensiveOutpatient');
      expect(TreatmentModality.PartialHospitalization).toBe('PartialHospitalization');
      expect(TreatmentModality.Inpatient).toBe('Inpatient');
      expect(TreatmentModality.CrisisIntervention).toBe('CrisisIntervention');
      expect(TreatmentModality.PeerSupport).toBe('PeerSupport');
      expect(TreatmentModality.Telehealth).toBe('Telehealth');
      expect(TreatmentModality.Other).toBe('Other');
    });
  });

  describe('SafetyPlanStatus', () => {
    it('should have all safety plan statuses', () => {
      expect(SafetyPlanStatus.Active).toBe('Active');
      expect(SafetyPlanStatus.NeedsUpdate).toBe('NeedsUpdate');
      expect(SafetyPlanStatus.Expired).toBe('Expired');
      expect(SafetyPlanStatus.NotApplicable).toBe('NotApplicable');
    });
  });

  describe('SubstanceCategory', () => {
    it('should have all substance categories', () => {
      expect(SubstanceCategory.Alcohol).toBe('Alcohol');
      expect(SubstanceCategory.Cannabis).toBe('Cannabis');
      expect(SubstanceCategory.Opioids).toBe('Opioids');
      expect(SubstanceCategory.Stimulants).toBe('Stimulants');
      expect(SubstanceCategory.Sedatives).toBe('Sedatives');
      expect(SubstanceCategory.Hallucinogens).toBe('Hallucinogens');
      expect(SubstanceCategory.Tobacco).toBe('Tobacco');
      expect(SubstanceCategory.Other).toBe('Other');
    });
  });

  describe('Part2ConsentType', () => {
    it('should have all 42 CFR Part 2 consent types', () => {
      expect(Part2ConsentType.GeneralDisclosure).toBe('GeneralDisclosure');
      expect(Part2ConsentType.RedisclosureProhibited).toBe('RedisclosureProhibited');
      expect(Part2ConsentType.MedicalEmergency).toBe('MedicalEmergency');
      expect(Part2ConsentType.Research).toBe('Research');
      expect(Part2ConsentType.CourtOrder).toBe('CourtOrder');
      expect(Part2ConsentType.AuditEvaluation).toBe('AuditEvaluation');
    });
  });
});

describe('Chronic Care Types', () => {
  describe('DiabetesType', () => {
    it('should have all diabetes types', () => {
      expect(DiabetesType.Type1).toBe('Type1');
      expect(DiabetesType.Type2).toBe('Type2');
      expect(DiabetesType.Gestational).toBe('Gestational');
      expect(DiabetesType.LADA).toBe('LADA');
      expect(DiabetesType.MODY).toBe('MODY');
      expect(DiabetesType.Other).toBe('Other');
    });
  });

  describe('NYHAClass', () => {
    it('should have all NYHA heart failure classes', () => {
      expect(NYHAClass.ClassI).toBe('ClassI');
      expect(NYHAClass.ClassII).toBe('ClassII');
      expect(NYHAClass.ClassIII).toBe('ClassIII');
      expect(NYHAClass.ClassIV).toBe('ClassIV');
    });
  });

  describe('GOLDStage', () => {
    it('should have all GOLD COPD stages', () => {
      expect(GOLDStage.Mild).toBe('Mild');
      expect(GOLDStage.Moderate).toBe('Moderate');
      expect(GOLDStage.Severe).toBe('Severe');
      expect(GOLDStage.VerySevere).toBe('VerySevere');
    });
  });

  describe('CKDStage', () => {
    it('should have all CKD stages', () => {
      expect(CKDStage.Stage1).toBe('Stage1');
      expect(CKDStage.Stage2).toBe('Stage2');
      expect(CKDStage.Stage3a).toBe('Stage3a');
      expect(CKDStage.Stage3b).toBe('Stage3b');
      expect(CKDStage.Stage4).toBe('Stage4');
      expect(CKDStage.Stage5).toBe('Stage5');
    });
  });

  describe('AlertSeverity', () => {
    it('should have all alert severities', () => {
      expect(AlertSeverity.Info).toBe('Info');
      expect(AlertSeverity.Warning).toBe('Warning');
      expect(AlertSeverity.Urgent).toBe('Urgent');
      expect(AlertSeverity.Critical).toBe('Critical');
    });
  });
});

describe('Pediatric Types', () => {
  describe('VaccineType', () => {
    it('should have all vaccine types', () => {
      expect(VaccineType.HepB).toBe('HepB');
      expect(VaccineType.RV).toBe('RV');
      expect(VaccineType.DTaP).toBe('DTaP');
      expect(VaccineType.Hib).toBe('Hib');
      expect(VaccineType.PCV13).toBe('PCV13');
      expect(VaccineType.IPV).toBe('IPV');
      expect(VaccineType.Influenza).toBe('Influenza');
      expect(VaccineType.MMR).toBe('MMR');
      expect(VaccineType.Varicella).toBe('Varicella');
      expect(VaccineType.HepA).toBe('HepA');
      expect(VaccineType.MenACWY).toBe('MenACWY');
      expect(VaccineType.Tdap).toBe('Tdap');
      expect(VaccineType.HPV).toBe('HPV');
      expect(VaccineType.MenB).toBe('MenB');
      expect(VaccineType.COVID19).toBe('COVID19');
      expect(VaccineType.Other).toBe('Other');
    });
  });

  describe('ImmunizationStatus', () => {
    it('should have all immunization statuses', () => {
      expect(ImmunizationStatus.Completed).toBe('Completed');
      expect(ImmunizationStatus.InProgress).toBe('InProgress');
      expect(ImmunizationStatus.Overdue).toBe('Overdue');
      expect(ImmunizationStatus.NotStarted).toBe('NotStarted');
      expect(ImmunizationStatus.Contraindicated).toBe('Contraindicated');
      expect(ImmunizationStatus.Declined).toBe('Declined');
    });
  });

  describe('DevelopmentalDomain', () => {
    it('should have all developmental domains', () => {
      expect(DevelopmentalDomain.GrossMotor).toBe('GrossMotor');
      expect(DevelopmentalDomain.FineMotor).toBe('FineMotor');
      expect(DevelopmentalDomain.Language).toBe('Language');
      expect(DevelopmentalDomain.Cognitive).toBe('Cognitive');
      expect(DevelopmentalDomain.SocialEmotional).toBe('SocialEmotional');
      expect(DevelopmentalDomain.SelfHelp).toBe('SelfHelp');
    });
  });

  describe('MilestoneStatus', () => {
    it('should have all milestone statuses', () => {
      expect(MilestoneStatus.NotYetExpected).toBe('NotYetExpected');
      expect(MilestoneStatus.Expected).toBe('Expected');
      expect(MilestoneStatus.Achieved).toBe('Achieved');
      expect(MilestoneStatus.Delayed).toBe('Delayed');
      expect(MilestoneStatus.AtRisk).toBe('AtRisk');
      expect(MilestoneStatus.Concerning).toBe('Concerning');
    });
  });

  describe('FeedingType', () => {
    it('should have all feeding types', () => {
      expect(FeedingType.Breastfeeding).toBe('Breastfeeding');
      expect(FeedingType.FormulaFeeding).toBe('FormulaFeeding');
      expect(FeedingType.Mixed).toBe('Mixed');
      expect(FeedingType.Solids).toBe('Solids');
      expect(FeedingType.TableFood).toBe('TableFood');
    });
  });
});
