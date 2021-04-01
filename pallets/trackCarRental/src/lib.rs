#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{decl_event, decl_module, decl_error, decl_storage, ensure,
	dispatch::{DispatchResult, DispatchError}, RuntimeDebug, traits::Get};
use frame_system::ensure_signed;
use frame_support::codec::{Encode, Decode};
use sp_std::vec::Vec;

// To store a struct in blockchain storage, you need to derive (or manually ipmlement) each of these traits.
#[derive(Encode, Decode, RuntimeDebug, Clone, PartialEq, Eq)]
pub struct Car {
    name: Vec<u8>,
}

pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// The maximum number of Drivers.
	type MaxDrivers: Get<usize>;
}

decl_storage! {
	trait Store for Module<T: Config> as TrackCarRentalModule {
		// All cars that has been created
		Cars get(fn cars): map hasher(blake2_128_concat) u32 => Option<Car>;

		/// Stores the next Car ID
	    NextCarId get(fn next_car_id): u32;

		// The set of all drivers. Stored as a single vec
		Drivers get(fn drivers): Vec<T::AccountId>;

		// Car reserved by drivers
		SubscribedCars get(fn subscribed_cars): double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) u32 => Option<Car>;
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
		DriverAdded(AccountId),
		CarCreated(AccountId, Car),
		CarSubscribed(AccountId, Car),
		CarRemovedForDriver(AccountId),
	}
);

decl_error! {
	/// Error for the nicks module.
	pub enum Error for Module<T: Config> {
		/// Maximum numbers of Drivers reached
		DriversLimitReached,
		/// Driver already Exist
		AlreadyExistingDriver,
		/// Sender is not a driver
		NotADriver,
		/// Car Ids reached its Max Value
		CarIdOverflow,
		/// Car doen't exist
		CarDoesNotExist
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const MaxDrivers: u32 = T::MaxDrivers::get() as u32;

		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn add_driver(origin) -> DispatchResult {
			// Verify First
			let new_driver = ensure_signed(origin)?;

			let mut drivers = Drivers::<T>::get();

			ensure!(drivers.len() < T::MaxDrivers::get(), Error::<T>::DriversLimitReached);

			match drivers.binary_search(&new_driver) {
				Ok(_) => Err(Error::<T>::AlreadyExistingDriver.into()),
				Err(index) => {
					// Write Last
					drivers.insert(index, new_driver.clone());
					Drivers::<T>::put(drivers);
					Self::deposit_event(RawEvent::DriverAdded(new_driver));
					Ok(())
				}
			}
		}

		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn create_car(origin, name: Vec<u8>) -> DispatchResult {
			let driver = ensure_signed(origin)?;

			let drivers = Drivers::<T>::get();
			ensure!(drivers.contains(&driver), Error::<T>::NotADriver);

			let current_id = Self::get_next_car_id()?;

			let new_car = Car { name };

			Cars::insert(current_id, new_car.clone());

			Self::deposit_event(RawEvent::CarCreated(driver, new_car));
			Ok(())
		}

		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn reserve_car(origin, model_name: Vec<u8>) -> DispatchResult {
			let driver = ensure_signed(origin)?;

			let drivers = Self::drivers();
			ensure!(drivers.contains(&driver), Error::<T>::NotADriver);

			let car_id = Self::get_car_id(model_name)?;
			let car: Car = Self::cars(car_id).ok_or(Error::<T>::CarDoesNotExist)?;

			SubscribedCars::<T>::insert(&driver, car_id, &car);

			Self::deposit_event(RawEvent::CarSubscribed(driver, car));

			Ok(())
		}

		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn remove_reserved_car(origin) -> DispatchResult {
			let driver = ensure_signed(origin)?;

			let drivers = Self::drivers();
			ensure!(drivers.contains(&driver), Error::<T>::NotADriver);

			SubscribedCars::<T>::remove_prefix(&driver);

			Self::deposit_event(RawEvent::CarRemovedForDriver(driver));

			Ok(())
		}
	}
}

impl<T: Config> Module<T> {
	fn get_next_car_id() -> sp_std::result::Result<u32, DispatchError> {
		NextCarId::try_mutate(|next_id| -> sp_std::result::Result<u32, DispatchError> {
			let current_id = *next_id;
			*next_id = next_id.checked_add(1).ok_or(Error::<T>::CarIdOverflow)?;
			Ok(current_id)
		})
	}

	fn get_car_id(model_name: Vec<u8>) -> sp_std::result::Result<u32, DispatchError> {
		for i in 0..Self::next_car_id() {
			let car = Self::cars(i);
			match car {
				Some(car) => {
					if car.name == model_name {
						 return Ok(i as u32);
					}
				}
			    None => {}
			}
		}
		Err(Error::<T>::CarDoesNotExist.into())
	}
}

