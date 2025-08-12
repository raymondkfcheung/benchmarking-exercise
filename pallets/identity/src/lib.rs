//! # Simplified Identity Pallet
//!
//! A simplified version of the Identity pallet designed for benchmarking exercises.
//! 
//! This pallet provides basic identity management functionality:
//! - Set identity information with configurable fields
//! - Clear identity information
//! - Request judgements from registrars
//! - Provide judgements as a registrar
//! - Force operations (admin functions)
//!
//! ## Overview
//!
//! This pallet allows users to set identity information that can be verified by registrars.
//! Users pay deposits for storing identity information, and registrars can provide judgements
//! about the validity of identities.
//!
//! ### Key Features
//! - **Identity Information**: Users can set display name, legal name, web, email etc.
//! - **Registrar System**: Trusted entities can verify identity information
//! - **Deposits**: Economic mechanism to prevent spam and ensure data quality
//! - **Judgements**: Registrars provide opinions on identity validity
//!
//! ## Benchmarking Focus
//!
//! This simplified version is designed to provide various benchmarking scenarios:
//! - Storage operations of different complexity
//! - Economic operations (deposits, slashing)
//! - Linear complexity based on data size
//! - Conditional logic branches
//! - Vector operations and bounded collections

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, ReservableCurrency, Get},
	BoundedVec,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_runtime::traits::{Zero, Saturating};
use sp_std::vec;

/// Maximum length for identity field data
pub const MAX_FIELD_LENGTH: u32 = 64;

/// Identity information that can be set by users
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct IdentityInfo {
	/// A reasonable display name for the controller of the account.
	pub display: BoundedVec<u8, ConstU32<MAX_FIELD_LENGTH>>,
	/// The full legal name in the local jurisdiction of the entity.
	pub legal: BoundedVec<u8, ConstU32<MAX_FIELD_LENGTH>>,
	/// A representative website field.
	pub web: BoundedVec<u8, ConstU32<MAX_FIELD_LENGTH>>,
	/// An email address.
	pub email: BoundedVec<u8, ConstU32<MAX_FIELD_LENGTH>>,
}

impl Default for IdentityInfo {
	fn default() -> Self {
		Self {
			display: BoundedVec::default(),
			legal: BoundedVec::default(),
			web: BoundedVec::default(),
			email: BoundedVec::default(),
		}
	}
}

impl IdentityInfo {
	/// Get the encoded size of this identity info
	pub fn encoded_size(&self) -> u32 {
		self.encode().len() as u32
	}
}


/// Judgement provided by registrars
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Judgement<Balance> {
	/// The default value; no opinion is held.
	Unknown,
	/// A judgement is being requested, with the given fee reserved.
	FeePaid(Balance),
	/// The target is known directly by the registrar and the registrar is confident that the
	/// identity is reasonable.
	Reasonable,
	/// The target is known directly by the registrar and the registrar is confident that the
	/// identity is good.
	KnownGood,
	/// The target is known directly by the registrar and the registrar is confident that the
	/// identity is erroneous.
	Erroneous,
	/// An erroneous identity may be corrected by governance.
	LowQuality,
}

impl<Balance> Judgement<Balance> {
	/// Returns true if this judgement is "sticky" (cannot be removed except by complete
	/// removal of the identity or by the registrar).
	pub fn is_sticky(&self) -> bool {
		matches!(self, Judgement::KnownGood | Judgement::Erroneous)
	}

	/// Returns true if this judgement has an associated deposit.
	pub fn has_deposit(&self) -> bool {
		matches!(self, Judgement::FeePaid(_))
	}
}


/// Information concerning the identity of the controller of an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Registration<Balance> {
	/// Information about the identity.
	pub info: IdentityInfo,
	/// Judgements from the registrars on this identity. Stored ordered by registrar index.
	pub judgements: BoundedVec<(u32, Judgement<Balance>), ConstU32<20>>,
	/// Amount reserved for the identity information.
	pub deposit: Balance,
}

impl<Balance: Zero + Saturating + Copy> Registration<Balance> {
	/// Calculate the total deposit for this registration
	pub fn total_deposit(&self) -> Balance {
		self.deposit
	}
}

/// Information about a registrar
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RegistrarInfo<Balance, AccountId> {
	/// The account of the registrar.
	pub account: AccountId,
	/// The fee required to be paid for a judgement to be given by this registrar.
	pub fee: Balance,
}

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type RegistrarIndex = u32;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The amount held on deposit for a registered identity.
		#[pallet::constant]
		type BasicDeposit: Get<BalanceOf<Self>>;

		/// The amount held on deposit per encoded byte for a registered identity.
		#[pallet::constant]
		type ByteDeposit: Get<BalanceOf<Self>>;

		/// Maximum number of registrars allowed in the system.
		#[pallet::constant]
		type MaxRegistrars: Get<u32>;

		/// The origin which may forcibly set or remove a name. Root can always do this.
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The origin which may add or remove registrars. Root can always do this.
		type RegistrarOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// Information that is pertinent to identify the entity behind an account.
	#[pallet::storage]
	pub type IdentityOf<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Registration<BalanceOf<T>>,
		OptionQuery,
	>;

	/// The set of registrars. Not expected to get very big as can only be added through a
	/// special origin (likely a council motion).
	#[pallet::storage]
	pub type Registrars<T: Config> = StorageValue<
		_,
		BoundedVec<Option<RegistrarInfo<BalanceOf<T>, T::AccountId>>, T::MaxRegistrars>,
		ValueQuery,
	>;

	/// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A name was set or reset (which will remove all judgements).
		IdentitySet { who: T::AccountId },
		/// A name was cleared, and the given balance returned.
		IdentityCleared { who: T::AccountId, deposit: BalanceOf<T> },
		/// A name was removed and the given balance slashed.
		IdentityKilled { who: T::AccountId, deposit: BalanceOf<T> },
		/// A judgement was asked from a registrar.
		JudgementRequested { who: T::AccountId, registrar_index: RegistrarIndex },
		/// A judgement was given by a registrar.
		JudgementGiven { target: T::AccountId, registrar_index: RegistrarIndex },
		/// A registrar was added.
		RegistrarAdded { registrar_index: RegistrarIndex },
	}

	/// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Account isn't found.
		NotFound,
		/// No identity found.
		NoIdentity,
		/// Fee is changed.
		FeeChanged,
		/// Sticky judgement.
		StickyJudgement,
		/// Judgement given.
		JudgementGiven,
		/// Invalid judgement.
		InvalidJudgement,
		/// The index is invalid.
		InvalidIndex,
		/// The target is invalid.
		InvalidTarget,
		/// Maximum amount of registrars reached. Cannot add any more.
		TooManyRegistrars,
		/// Error that occurs when there is an issue paying for judgement.
		JudgementPaymentFailed,
	}

	/// Dispatchable functions allow users to interact with the pallet and invoke state changes.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a registrar to the system.
		///
		/// The dispatch origin for this call must be `T::RegistrarOrigin`.
		///
		/// - `account`: the account of the registrar.
		///
		/// Emits `RegistrarAdded` if successful.
		pub fn add_registrar(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResult {
			T::RegistrarOrigin::ensure_origin(origin)?;

			let (i, _registrar_count) = Registrars::<T>::try_mutate(
				|registrars| -> Result<(RegistrarIndex, usize), DispatchError> {
					registrars
						.try_push(Some(RegistrarInfo {
							account,
							fee: Zero::zero(),
						}))
						.map_err(|_| Error::<T>::TooManyRegistrars)?;
					Ok(((registrars.len() - 1) as RegistrarIndex, registrars.len()))
				},
			)?;

			Self::deposit_event(Event::RegistrarAdded { registrar_index: i });
			Ok(())
		}

		/// Set an account's identity information and reserve the appropriate deposit.
		///
		/// If the account already has identity information, the deposit is taken as part payment
		/// for the new deposit.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `display`: The display name.
		/// - `legal`: The legal name.
		/// - `web`: The web address.
		/// - `email`: The email address.
		///
		/// Emits `IdentitySet` if successful.
		pub fn set_identity(
			origin: OriginFor<T>,
			display: BoundedVec<u8, ConstU32<MAX_FIELD_LENGTH>>,
			legal: BoundedVec<u8, ConstU32<MAX_FIELD_LENGTH>>,
			web: BoundedVec<u8, ConstU32<MAX_FIELD_LENGTH>>,
			email: BoundedVec<u8, ConstU32<MAX_FIELD_LENGTH>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let info = IdentityInfo {
				display,
				legal,
				web,
				email,
			};

			let mut id = match IdentityOf::<T>::get(&sender) {
				Some(mut id) => {
					// Only keep non-positive judgements (sticky judgements).
					id.judgements.retain(|j| j.1.is_sticky());
					id.info = info;
					id
				},
				None => Registration {
					info,
					judgements: BoundedVec::default(),
					deposit: Zero::zero(),
				},
			};

			let new_deposit = Self::calculate_identity_deposit(&id.info);
			let old_deposit = id.deposit;
			Self::rejig_deposit(&sender, old_deposit, new_deposit)?;

			id.deposit = new_deposit;
			IdentityOf::<T>::insert(&sender, id);
			Self::deposit_event(Event::IdentitySet { who: sender });

			Ok(())
		}

		/// Clear an account's identity info and return all deposits.
		///
		/// Payment: All reserved balances on the account are returned.
		///
		/// The dispatch origin for this call must be _Signed_ and the sender must have a registered
		/// identity.
		///
		/// Emits `IdentityCleared` if successful.
		pub fn clear_identity(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let id = IdentityOf::<T>::take(&sender).ok_or(Error::<T>::NoIdentity)?;
			let deposit = id.total_deposit();

			let err_amount = T::Currency::unreserve(&sender, deposit);
			debug_assert!(err_amount.is_zero());

			Self::deposit_event(Event::IdentityCleared { who: sender, deposit });
			Ok(())
		}

		/// Request a judgement from a registrar.
		///
		/// Payment: At most `max_fee` will be reserved for payment to the registrar if judgement
		/// given.
		///
		/// The dispatch origin for this call must be _Signed_ and the sender must have a
		/// registered identity.
		///
		/// - `reg_index`: The index of the registrar whose judgement is requested.
		/// - `max_fee`: The maximum fee that may be paid.
		///
		/// Emits `JudgementRequested` if successful.
		pub fn request_judgement(
			origin: OriginFor<T>,
			reg_index: RegistrarIndex,
			max_fee: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let registrars = Registrars::<T>::get();
			let registrar = registrars
				.get(reg_index as usize)
				.and_then(Option::as_ref)
				.ok_or(Error::<T>::InvalidIndex)?;
			ensure!(max_fee >= registrar.fee, Error::<T>::FeeChanged);
			let mut id = IdentityOf::<T>::get(&sender).ok_or(Error::<T>::NoIdentity)?;

			let item = (reg_index, Judgement::FeePaid(registrar.fee));
			match id.judgements.binary_search_by_key(&reg_index, |x| x.0) {
				Ok(i) =>
					if id.judgements[i].1.is_sticky() {
						return Err(Error::<T>::StickyJudgement.into())
					} else {
						id.judgements[i] = item
					},
				Err(i) =>
					id.judgements.try_insert(i, item).map_err(|_| Error::<T>::TooManyRegistrars)?,
			}

			T::Currency::reserve(&sender, registrar.fee)?;
			IdentityOf::<T>::insert(&sender, id);

			Self::deposit_event(Event::JudgementRequested {
				who: sender,
				registrar_index: reg_index,
			});

			Ok(())
		}

		/// Provide a judgement for an account's identity.
		///
		/// The dispatch origin for this call must be _Signed_ and the sender must be the account
		/// of the registrar whose index is `reg_index`.
		///
		/// - `reg_index`: the index of the registrar whose judgement is being made.
		/// - `target`: the account whose identity the judgement is upon. This must be an account
		///   with a registered identity.
		/// - `judgement_type`: the type of judgement (0=Unknown, 1=Reasonable, 2=KnownGood, 3=Erroneous, 4=LowQuality).
		///
		/// Emits `JudgementGiven` if successful.
		pub fn provide_judgement(
			origin: OriginFor<T>,
			reg_index: RegistrarIndex,
			target: T::AccountId,
			judgement_type: u8,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			
			// Convert u8 to Judgement
			let judgement = match judgement_type {
				0 => Judgement::Unknown,
				1 => Judgement::Reasonable,
				2 => Judgement::KnownGood,
				3 => Judgement::Erroneous,
				4 => Judgement::LowQuality,
				_ => return Err(Error::<T>::InvalidJudgement.into()),
			};
			
			ensure!(!judgement.has_deposit(), Error::<T>::InvalidJudgement);
			
			Registrars::<T>::get()
				.get(reg_index as usize)
				.and_then(Option::as_ref)
				.filter(|r| r.account == sender)
				.ok_or(Error::<T>::InvalidIndex)?;
				
			let mut id = IdentityOf::<T>::get(&target).ok_or(Error::<T>::InvalidTarget)?;

			let item = (reg_index, judgement);
			match id.judgements.binary_search_by_key(&reg_index, |x| x.0) {
				Ok(position) => {
					if let Judgement::FeePaid(fee) = id.judgements[position].1 {
						let _remainder = T::Currency::repatriate_reserved(
							&target,
							&sender,
							fee,
							frame_support::traits::BalanceStatus::Free,
						);
					}
					id.judgements[position] = item
				},
				Err(position) => id
					.judgements
					.try_insert(position, item)
					.map_err(|_| Error::<T>::TooManyRegistrars)?,
			}

			IdentityOf::<T>::insert(&target, id);
			Self::deposit_event(Event::JudgementGiven { target, registrar_index: reg_index });

			Ok(())
		}

		/// Remove an account's identity and sub-account information and slash the deposits.
		///
		/// Payment: Reserved balances from `set_identity` are slashed.
		///
		/// The dispatch origin for this call must match `T::ForceOrigin`.
		///
		/// - `target`: the account whose identity the judgement is upon. This must be an account
		///   with a registered identity.
		///
		/// Emits `IdentityKilled` if successful.
		pub fn kill_identity(
			origin: OriginFor<T>,
			target: T::AccountId,
		) -> DispatchResult {
			T::ForceOrigin::ensure_origin(origin)?;

			// Grab their deposit (and check that they have one).
			let id = IdentityOf::<T>::take(&target).ok_or(Error::<T>::NoIdentity)?;
			let deposit = id.total_deposit();
			
			// Slash their deposit from them.
			let _imbalance = T::Currency::slash_reserved(&target, deposit);

			Self::deposit_event(Event::IdentityKilled { who: target, deposit });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Get the identity information for an account.
		pub fn identity_of(who: &T::AccountId) -> Option<Registration<BalanceOf<T>>> {
			IdentityOf::<T>::get(who)
		}

		/// Get the list of registrars.
		pub fn registrars() -> BoundedVec<Option<RegistrarInfo<BalanceOf<T>, T::AccountId>>, T::MaxRegistrars> {
			Registrars::<T>::get()
		}

		/// Calculate the deposit required for an identity.
		fn calculate_identity_deposit(info: &IdentityInfo) -> BalanceOf<T> {
			let bytes = info.encoded_size();
			let byte_deposit = T::ByteDeposit::get().saturating_mul(BalanceOf::<T>::from(bytes));
			T::BasicDeposit::get().saturating_add(byte_deposit)
		}

		/// Take the `current` deposit that `who` is holding, and update it to a `new` one.
		fn rejig_deposit(
			who: &T::AccountId,
			current: BalanceOf<T>,
			new: BalanceOf<T>,
		) -> DispatchResult {
			if new > current {
				T::Currency::reserve(who, new - current)?;
			} else if new < current {
				let err_amount = T::Currency::unreserve(who, current - new);
				debug_assert!(err_amount.is_zero());
			}
			Ok(())
		}
	}
}