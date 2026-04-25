use futures::{FutureExt, channel::oneshot};
use gpui::{Context, Task};
use std::{marker::PhantomData, time::Duration};

pub struct DebouncedDelay<E: 'static> {
    task: Option<Task<()>>,
    cancel_channel: Option<oneshot::Sender<()>>,
    _phantom_data: PhantomData<E>,
}

impl<E: 'static> Default for DebouncedDelay<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: 'static> DebouncedDelay<E> {
    pub fn new() -> Self {
        Self {
            task: None,
            cancel_channel: None,
            _phantom_data: PhantomData,
        }
    }

    /// pending 디바운스를 즉시 취소. 이미 fire 된 작업은 영향 받지 않으며
    /// 아직 timer 대기 중인 task 는 oneshot 신호로 일찍 종료된다.
    /// 호출자가 다음 단계에서 동기 저장 등 명시 동작을 수행하기 직전에 사용.
    pub fn cancel(&mut self) {
        if let Some(channel) = self.cancel_channel.take() {
            _ = channel.send(());
        }
        self.task = None;
    }

    pub fn fire_new<F>(&mut self, delay: Duration, cx: &mut Context<E>, func: F)
    where
        F: 'static + Send + FnOnce(&mut E, &mut Context<E>) -> Task<()>,
    {
        if let Some(channel) = self.cancel_channel.take() {
            _ = channel.send(());
        }

        let (sender, mut receiver) = oneshot::channel::<()>();
        self.cancel_channel = Some(sender);

        let previous_task = self.task.take();
        self.task = Some(cx.spawn(async move |entity, cx| {
            let mut timer = cx.background_executor().timer(delay).fuse();
            if let Some(previous_task) = previous_task {
                previous_task.await;
            }

            futures::select_biased! {
                _ = receiver => return,
                _ = timer => {}
            }

            if let Ok(task) = entity.update(cx, |project, cx| (func)(project, cx)) {
                task.await;
            }
        }));
    }
}
