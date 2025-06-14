use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableCell};
use std::cell::RefCell;
use std::time::Duration;

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    pub static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    pub static REQUEST_ID: RefCell<StableCell<u64, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableCell::init(memory_manager.get(MemoryId::new(0)), 0u64).expect("Failed to initialize request id"))
    });

    pub static STATE: RefCell<StableCell<u8, Memory>> = with_memory_manager(|memory_manager| {
        RefCell::new(StableCell::init(memory_manager.get(MemoryId::new(1)), 1u8).expect("Failed to initialize state"))
    });
}

pub fn with_memory_manager<R>(f: impl FnOnce(&MemoryManager<DefaultMemoryImpl>) -> R) -> R {
    MEMORY_MANAGER.with(|cell| f(&cell.borrow()))
}

#[ic_cdk::update]
async fn timer() -> Result<String, String> {
    // retry 1000 times
    for _ in 0..1000 {
        ic_cdk::println!("Checking state...");
        // call get_state through inter-canister call, to give up control
        match ic_cdk::call::<((),), (u8,)>(ic_cdk::id(), "get_state", ((),))
            .await
            .map_err(|e| e.1)?
            // Access the first element of the tuple, which is the `Result<BlockIndex, TransferError>`, for further processing.
            .0
        {
            // If the state is a multiple of 2, we can proceed with the complex code
            state if state % 2 == 0 => {
                ic_cdk::println!("State is a multiple of 2: {}", state);
                // state is met. continue with complex code
                // ...
                // no need to set another timer
                return Ok("All done".to_string());
            }
            _ => {} // retry again
        }
    }

    Err("Timed out".to_string())
}

#[ic_cdk::update]
fn timer_async() -> u64 {
    let request_id = REQUEST_ID.with(|cell| {
        let mut id = cell.borrow_mut();
        let current_id = id.get();
        let new_id = current_id.wrapping_add(1);
        _ = id.set(new_id);
        new_id
    });

    ic_cdk_timers::set_timer(Duration::from_millis(0), || {
        ic_cdk::spawn(async {
            check_state_timer().await;
        });
    });

    request_id
}

#[ic_cdk::query]
fn get_state() -> u8 {
    STATE.with(|cell| *cell.borrow().get())
}

async fn check_state_timer() {
    ic_cdk::println!("Checking state...");
    let state = get_state();
    if state % 2 == 0 {
        ic_cdk::println!("State is a multiple of 2: {}", state);
        // state is met. continue with complex code
        // ...
        // no need to set another timer
        return;
    }
    // state not met, set another timer
    ic_cdk_timers::set_timer(Duration::from_millis(2000), || {
        ic_cdk::spawn(async {
            check_state_timer().await;
        });
    });
}

#[ic_cdk::update]
fn incr_state() -> u8 {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let current_state = state.get();
        let new_state = current_state.wrapping_add(1);
        _ = state.set(new_state);
        new_state
    })
}

ic_cdk::export_candid!();
