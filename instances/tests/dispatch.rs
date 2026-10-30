#![cfg(feature = "runtime")]
use miresidian_codec::InvocationOutput;
use miresidian_instances::{ExecutionMode, invoke_multi_channel, plan_multi_channel};
use std::{
    sync::{Condvar, Mutex},
    time::Duration,
};
#[test]
fn general_parallel_dispatch_overlaps_independent_calls_and_aligns_outputs() {
    let barrier = (Mutex::new(0), Condvar::new());
    let slots = vec![Some(vec!["a".into()]), None, Some(vec!["b".into()])];
    let result = invoke_multi_channel(&slots, &[None], ExecutionMode::Parallel, |input| {
        let mut started = barrier.0.lock().unwrap();
        *started += 1;
        barrier.1.notify_all();
        let (_guard, t) = barrier
            .1
            .wait_timeout_while(started, Duration::from_secs(2), |n| *n < 2)
            .unwrap();
        assert!(!t.timed_out(), "parallel calls did not overlap");
        Ok(InvocationOutput::new(input.to_vec()))
    })
    .unwrap();
    assert_eq!(result[0].as_ref().unwrap().as_slice(), ["a"]);
    assert!(result[1].is_none());
    assert_eq!(result[2].as_ref().unwrap().as_slice(), ["b"]);
}
#[test]
fn async_and_sync_frontends_share_skip_and_broadcast_plans() {
    let slots = vec![Some(vec!["x".into()]), Some(vec![]), Some(vec!["".into()])];
    let plans = plan_multi_channel(&slots, &[None], ExecutionMode::Sequential).unwrap();
    assert_eq!(plans, vec![Some(vec!["x".into()]), None, None]);
    assert!(
        plan_multi_channel(
            &[Some(vec!["x".into()]), Some(vec!["y".into()])],
            &[None],
            ExecutionMode::Broadcast
        )
        .is_err()
    );
}
