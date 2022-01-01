use crate::Test;
use io_uring::squeue::SqeCommonOptions;
use io_uring::{opcode, IoUring};

pub fn test_nop(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require! {
        test;
    }

    println!("test nop");

    let nop_e = opcode::Nop::new().build().user_data(0x42);

    unsafe {
        let mut queue = ring.submission();
        queue.push(&nop_e).expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x42);
    assert_eq!(cqes[0].result(), 0);

    Ok(())
}

#[cfg(feature = "unstable")]
pub fn test_nop_prepare(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require! {
        test;
    }

    println!("test nop_prepare");

    let nop = opcode::Nop::new();
    let opt = SqeCommonOptions::default().user_data(0x42);

    unsafe {
        let mut queue = ring.submission();
        queue.push_command(&nop, Some(&opt)).expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x42);
    assert_eq!(cqes[0].result(), 0);

    Ok(())
}

#[cfg(feature = "unstable")]
pub fn test_nop_prepare_sqe(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require! {
        test;
    }

    println!("test nop_prepare_sqe");

    let opt = SqeCommonOptions::default().user_data(0x42);

    unsafe {
        let mut queue = ring.submission();
        let sqe = queue.get_available_sqe(0).unwrap();
        let nop_sqe: &mut opcode::NopSqe = sqe.into();
        nop_sqe.prepare();
        opt.set(nop_sqe.get_mut_sqe());
        queue.move_forward(1);
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x42);
    assert_eq!(cqes[0].result(), 0);

    Ok(())
}

#[cfg(feature = "unstable")]
pub fn test_batch(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    use std::mem::MaybeUninit;

    require! {
        test;
    }

    println!("test batch");

    assert!(ring.completion().is_empty());

    unsafe {
        let sqes = vec![opcode::Nop::new().build().user_data(0x09); 5];
        let mut sq = ring.submission();

        assert_eq!(sq.capacity(), 8);

        sq.push_multiple(&sqes).unwrap();

        assert_eq!(sq.len(), 5);

        let ret = sq.push_multiple(&sqes);
        assert!(ret.is_err());

        assert_eq!(sq.len(), 5);

        sq.push_multiple(&sqes[..3]).unwrap();
    }

    ring.submit_and_wait(8)?;

    let mut cqes = (0..10).map(|_| MaybeUninit::uninit()).collect::<Vec<_>>();

    let cqes = ring.completion().fill(&mut cqes);

    assert_eq!(cqes.len(), 8);

    for entry in cqes {
        assert_eq!(entry.user_data(), 0x09);
    }

    Ok(())
}

pub fn test_queue_split(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require! {
        test;
    }

    println!("test queue_split");

    let (submitter, mut sq, mut cq) = ring.split();

    assert!(sq.is_empty());

    for _ in 0..sq.capacity() {
        unsafe {
            sq.push(&opcode::Nop::new().build()).expect("queue is full");
        }
    }

    assert!(sq.is_full());

    sq.sync();

    assert_eq!(submitter.submit()?, sq.capacity());
    assert!(sq.is_full());
    sq.sync();
    assert!(sq.is_empty());

    assert!(cq.is_empty());
    cq.sync();
    assert_eq!(cq.len(), sq.capacity());
    assert_eq!(cq.by_ref().count(), sq.capacity());

    cq.sync();

    Ok(())
}

pub fn test_debug_print(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require! {
        test;
    }

    println!("test debug_print");

    let mut sq = ring.submission();
    let num_to_sub = sq.capacity();
    for _ in 0..num_to_sub {
        unsafe {
            sq.push(&opcode::Nop::new().build().user_data(0x42))
                .expect("queue is full");
        }
    }
    println!("Full: {:?}", sq);
    drop(sq);

    ring.submit_and_wait(num_to_sub)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), num_to_sub);
    for cqe in cqes {
        assert_eq!(cqe.user_data(), 0x42);
        assert_eq!(cqe.result(), 0);
    }
    println!("Empty: {:?}", ring.submission());

    Ok(())
}
