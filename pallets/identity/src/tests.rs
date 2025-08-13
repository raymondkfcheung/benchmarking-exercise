use crate::{mock::*, pallet::JudgementsDoubleMap, Error, Event, IdentityInfo, Judgement};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use sp_runtime::traits::Zero;

#[test]
fn set_identity_works() {
	new_test_ext().execute_with(|| {
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: b"legal".to_vec().try_into().unwrap(),
			web: b"web".to_vec().try_into().unwrap(),
			email: b"email".to_vec().try_into().unwrap(),
		};

		// Set identity for account 1
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display.clone(),
			info.legal.clone(),
			info.web.clone(),
			info.email.clone(),
		));

		// Check storage
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.info, info);
		assert!(!registration.deposit.is_zero());
		assert_eq!(registration.judgements.len(), 0);
		assert_eq!(registration.judgements_count_double_map, 0);

		// Check event
		System::assert_last_event(Event::IdentitySet { who: 1 }.into());
	});
}

#[test]
fn clear_identity_works() {
	new_test_ext().execute_with(|| {
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};

		// Set identity first
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));
		let deposit = Identity::identity_of(&1).unwrap().deposit;

		// Clear identity
		assert_ok!(Identity::clear_identity(RuntimeOrigin::signed(1)));

		// Check storage is cleared
		assert!(Identity::identity_of(&1).is_none());

		// Check event
		System::assert_last_event(Event::IdentityCleared { who: 1, deposit }.into());
	});
}

#[test]
fn clear_identity_fails_without_identity() {
	new_test_ext().execute_with(|| {
		// Try to clear non-existent identity
		assert_noop!(Identity::clear_identity(RuntimeOrigin::signed(1)), Error::<Test>::NoIdentity);
	});
}

#[test]
fn provide_judgement_inline_works() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));

		// Provide judgement (2 = KnownGood) with judgement_id 0
		assert_ok!(Identity::provide_judgement_inline(
			RuntimeOrigin::root(),
			0, // judgement_id
			1, // target
			2  // judgement_type
		));

		// Check storage
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 1);
		assert_eq!(registration.judgements[0], (0, Judgement::KnownGood));

		// Check event
		System::assert_last_event(Event::JudgementGiven { target: 1 }.into());
	});
}

#[test]
fn provide_judgement_inline_fails_without_identity() {
	new_test_ext().execute_with(|| {
		// Try to provide judgement for non-existent identity
		assert_noop!(
			Identity::provide_judgement_inline(RuntimeOrigin::root(), 0, 1, 2),
			Error::<Test>::InvalidTarget
		);
	});
}

#[test]
fn provide_judgement_inline_respects_sticky_judgements() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));

		// Provide sticky judgement (2 = KnownGood) with judgement_id 0
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 0, 1, 2));

		// Try to override same judgement_id with different judgement - should fail
		assert_noop!(
			Identity::provide_judgement_inline(RuntimeOrigin::root(), 0, 1, 1),
			Error::<Test>::StickyJudgement
		);
	});
}

#[test]
fn set_identity_clears_non_sticky_judgement() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display.clone(),
			info.legal.clone(),
			info.web.clone(),
			info.email.clone(),
		));

		// Provide non-sticky judgement (1 = Reasonable) with judgement_id 0
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 0, 1, 1));
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 1);
		assert_eq!(registration.judgements[0], (0, Judgement::Reasonable));

		// Update identity - should clear non-sticky judgement
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			b"new_display".to_vec().try_into().unwrap(),
			info.legal,
			info.web,
			info.email,
		));

		// Non-sticky judgement should be cleared
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 0);
	});
}

#[test]
fn set_identity_preserves_sticky_judgement() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display.clone(),
			info.legal.clone(),
			info.web.clone(),
			info.email.clone(),
		));

		// Provide sticky judgement (2 = KnownGood) with judgement_id 0
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 0, 1, 2));
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 1);
		assert_eq!(registration.judgements[0], (0, Judgement::KnownGood));

		// Update identity - should preserve sticky judgement
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			b"new_display".to_vec().try_into().unwrap(),
			info.legal,
			info.web,
			info.email,
		));

		// Sticky judgement should be preserved
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 1);
		assert_eq!(registration.judgements[0], (0, Judgement::KnownGood));
	});
}

#[test]
fn deposit_calculation_works() {
	new_test_ext().execute_with(|| {
		// Test with different sized data
		let small_info = IdentityInfo {
			display: b"a".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};

		let large_info = IdentityInfo {
			display: b"a_much_longer_display_name_that_takes_up_more_bytes"
				.to_vec()
				.try_into()
				.unwrap(),
			legal: b"legal_name".to_vec().try_into().unwrap(),
			web: b"https://example.com".to_vec().try_into().unwrap(),
			email: b"test@example.com".to_vec().try_into().unwrap(),
		};

		// Set small identity
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			small_info.display,
			small_info.legal,
			small_info.web,
			small_info.email,
		));
		let small_deposit = Identity::identity_of(&1).unwrap().deposit;

		// Clear and set large identity
		assert_ok!(Identity::clear_identity(RuntimeOrigin::signed(1)));
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			large_info.display,
			large_info.legal,
			large_info.web,
			large_info.email,
		));
		let large_deposit = Identity::identity_of(&1).unwrap().deposit;

		// Large deposit should be greater than small deposit due to byte deposit
		assert!(large_deposit > small_deposit);
	});
}

#[test]
fn multiple_judgements_work() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));

		// Add multiple judgements with different IDs
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 5, 1, 1)); // Reasonable
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 1, 1, 2)); // KnownGood
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 10, 1, 3)); // Erroneous
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 0, 1, 4)); // LowQuality

		// Check storage - should be sorted by ID
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 4);
		assert_eq!(registration.judgements[0], (0, Judgement::LowQuality));
		assert_eq!(registration.judgements[1], (1, Judgement::KnownGood));
		assert_eq!(registration.judgements[2], (5, Judgement::Reasonable));
		assert_eq!(registration.judgements[3], (10, Judgement::Erroneous));
	});
}

#[test]
fn judgement_update_works() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));

		// Add initial judgement
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 5, 1, 1)); // Reasonable
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 1);
		assert_eq!(registration.judgements[0], (5, Judgement::Reasonable));

		// Update same judgement_id with different judgement
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 5, 1, 4)); // LowQuality
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 1);
		assert_eq!(registration.judgements[0], (5, Judgement::LowQuality));
	});
}

#[test]
fn mixed_sticky_non_sticky_judgements() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display.clone(),
			info.legal.clone(),
			info.web.clone(),
			info.email.clone(),
		));

		// Add mix of sticky and non-sticky judgements
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 1, 1, 1)); // Reasonable (non-sticky)
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 2, 1, 2)); // KnownGood (sticky)
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 3, 1, 3)); // Erroneous (sticky)
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 4, 1, 4)); // LowQuality (non-sticky)

		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 4);

		// Update identity - should only keep sticky judgements
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			b"new_display".to_vec().try_into().unwrap(),
			info.legal,
			info.web,
			info.email,
		));

		// Only sticky judgements should remain
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 2);
		assert_eq!(registration.judgements[0], (2, Judgement::KnownGood));
		assert_eq!(registration.judgements[1], (3, Judgement::Erroneous));
	});
}

#[test]
fn too_many_judgements_error() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));

		// Add judgements up to the maximum (20)
		for i in 0..20 {
			assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), i, 1, 1));
		}

		// Verify we've reached the limit
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 20);

		// Try to add one more judgement - should fail
		assert_noop!(
			Identity::provide_judgement_inline(RuntimeOrigin::root(), 20, 1, 1),
			Error::<Test>::TooManyJudgements
		);
	});
}

#[test]
fn inline_storage_pattern_works() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));

		// Add judgements using inline storage
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 5, 1, 1)); // Reasonable
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 1, 1, 2)); // KnownGood
		assert_ok!(Identity::provide_judgement_inline(RuntimeOrigin::root(), 10, 1, 3)); // Erroneous

		// Check inline storage (BoundedVec in Registration)
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 3);
		assert_eq!(registration.judgements[0], (1, Judgement::KnownGood));
		assert_eq!(registration.judgements[1], (5, Judgement::Reasonable));
		assert_eq!(registration.judgements[2], (10, Judgement::Erroneous));

		// Verify double map is still empty (since we only used inline)
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 1), None);
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 5), None);
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 10), None);

		// Clear identity using the unified method
		assert_ok!(Identity::clear_identity(RuntimeOrigin::signed(1)));

		// Verify inline storage is cleared
		assert!(Identity::identity_of(&1).is_none());
	});
}

#[test]
fn double_map_storage_pattern_works() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));

		// Add judgements using double map storage
		assert_ok!(Identity::provide_judgement_double_map(RuntimeOrigin::root(), 5, 1, 1)); // Reasonable
		assert_ok!(Identity::provide_judgement_double_map(RuntimeOrigin::root(), 1, 1, 2)); // KnownGood
		assert_ok!(Identity::provide_judgement_double_map(RuntimeOrigin::root(), 10, 1, 3)); // Erroneous

		// Check double map storage
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 1), Some(Judgement::KnownGood));
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 5), Some(Judgement::Reasonable));
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 10), Some(Judgement::Erroneous));
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 99), None); // Non-existent

		// Verify inline storage is still empty (since we only used double map)
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 0);

		// Clear identity using the unified method
		assert_ok!(Identity::clear_identity(RuntimeOrigin::signed(1)));

		// Verify both storages are cleared
		assert!(Identity::identity_of(&1).is_none());
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 1), None);
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 5), None);
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 10), None);
	});
}

#[test]
fn double_map_counter_tracks_correctly() {
	new_test_ext().execute_with(|| {
		// Setup: set identity
		let info = IdentityInfo {
			display: b"display".to_vec().try_into().unwrap(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		};
		assert_ok!(Identity::set_identity(
			RuntimeOrigin::signed(1),
			info.display,
			info.legal,
			info.web,
			info.email,
		));

		// Initial counter should be 0
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements_count_double_map, 0);

		// Add judgements using double map
		assert_ok!(Identity::provide_judgement_double_map(RuntimeOrigin::root(), 1, 1, 1)); // New
		assert_ok!(Identity::provide_judgement_double_map(RuntimeOrigin::root(), 2, 1, 2)); // New
		assert_ok!(Identity::provide_judgement_double_map(RuntimeOrigin::root(), 3, 1, 3)); // New

		// Counter should be 3
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements_count_double_map, 3);

		// Replace existing judgement (should not increment)
		assert_ok!(Identity::provide_judgement_double_map(RuntimeOrigin::root(), 1, 1, 4)); // Replace

		// Counter should still be 3
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements_count_double_map, 3);

		// Verify double map contents
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 1), Some(Judgement::LowQuality));
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 2), Some(Judgement::KnownGood));
		assert_eq!(JudgementsDoubleMap::<Test>::get(&1, 3), Some(Judgement::Erroneous));
	});
}
