use crate::{mock::*, Error, Event, IdentityInfo, Judgement, RegistrarInfo};
use frame_support::{
	assert_noop, assert_ok,
	BoundedVec,
};
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
fn add_registrar_works() {
	new_test_ext().execute_with(|| {
		// Add registrar
		assert_ok!(Identity::add_registrar(RuntimeOrigin::root(), 10));

		// Check storage
		let registrars = Identity::registrars();
		assert_eq!(registrars.len(), 1);
		assert_eq!(
			registrars[0],
			Some(RegistrarInfo { account: 10, fee: 0 })
		);

		// Check event
		System::assert_last_event(Event::RegistrarAdded { registrar_index: 0 }.into());
	});
}

#[test]
fn request_judgement_works() {
	new_test_ext().execute_with(|| {
		// Setup: add registrar and set identity
		assert_ok!(Identity::add_registrar(RuntimeOrigin::root(), 10));
		
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

		// Request judgement
		assert_ok!(Identity::request_judgement(RuntimeOrigin::signed(1), 0, 100));

		// Check storage
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements.len(), 1);
		assert_eq!(registration.judgements[0], (0, Judgement::FeePaid(0)));

		// Check event
		System::assert_last_event(Event::JudgementRequested { who: 1, registrar_index: 0 }.into());
	});
}

#[test]
fn provide_judgement_works() {
	new_test_ext().execute_with(|| {
		// Setup: add registrar, set identity, request judgement
		assert_ok!(Identity::add_registrar(RuntimeOrigin::root(), 10));
		
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
		assert_ok!(Identity::request_judgement(RuntimeOrigin::signed(1), 0, 100));

		// Provide judgement (2 = KnownGood)
		assert_ok!(Identity::provide_judgement(
			RuntimeOrigin::signed(10),
			0,
			1,
			2
		));

		// Check storage
		let registration = Identity::identity_of(&1).unwrap();
		assert_eq!(registration.judgements[0], (0, Judgement::KnownGood));

		// Check event
		System::assert_last_event(Event::JudgementGiven { target: 1, registrar_index: 0 }.into());
	});
}

#[test]
fn kill_identity_works() {
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

		// Kill identity
		assert_ok!(Identity::kill_identity(RuntimeOrigin::root(), 1));

		// Check storage is cleared
		assert!(Identity::identity_of(&1).is_none());

		// Check that IdentityKilled event was emitted (may not be the last event due to balance operations)
		System::assert_has_event(Event::IdentityKilled { who: 1, deposit }.into());
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