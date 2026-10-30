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
    assert_eq!(
        plan_multi_channel(
            &[Some(vec!["x".into()]), Some(vec!["y".into()])],
            &[None],
            ExecutionMode::Broadcast
        )
        .unwrap(),
        vec![Some(vec!["x".into()]), Some(vec!["y".into()])]
    );
}

#[test]
fn broadcast_executes_channel_one_once_and_clones_the_complete_output() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = AtomicUsize::new(0);
    let slots = vec![
        Some(vec!["10".into()]),
        Some(vec!["20".into()]),
        None,
        Some(vec!["30".into()]),
        Some(vec!["".into()]),
    ];
    let output = invoke_multi_channel(&slots, &[None], ExecutionMode::Broadcast, |input| {
        calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(input, ["10"]);
        Ok(InvocationOutput::new(vec![
            "R".into(),
            "second field".into(),
        ]))
    })
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let expected = Some(InvocationOutput::new(vec![
        "R".into(),
        "second field".into(),
    ]));
    assert_eq!(
        output,
        vec![expected.clone(), expected.clone(), None, expected, None]
    );
}
#[test]
fn invalid_channel_one_skips_the_entire_broadcast_without_fallback() {
    for first in [
        None,
        Some(vec![]),
        Some(vec!["".into()]),
        Some(vec!["a".into(), "extra".into()]),
    ] {
        let output = invoke_multi_channel(
            &[first, Some(vec!["valid".into()])],
            &[None],
            ExecutionMode::Broadcast,
            |_| panic!("invalid channel 1 must not invoke any channel"),
        )
        .unwrap();
        assert_eq!(output, vec![None, None]);
    }
    assert!(
        invoke_multi_channel(&[], &[None], ExecutionMode::Broadcast, |_| panic!(
            "empty broadcast"
        ))
        .unwrap()
        .is_empty()
    );
}
#[test]
fn broadcast_source_error_is_reported_once_without_running_other_inputs() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = AtomicUsize::new(0);
    let error = invoke_multi_channel(
        &[Some(vec!["bad".into()]), Some(vec!["good".into()])],
        &[None],
        ExecutionMode::Broadcast,
        |input| {
            calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(input, ["bad"]);
            Err(miresidian_codec::MiresidianError::Failure(
                "source failed".into(),
            ))
        },
    )
    .unwrap_err();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(error.to_string().contains("通道 1"));
}
