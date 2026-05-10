use kameo::actor::{Actor, ActorRef, Spawn};
use kameo::error::Infallible;
use kameo::message::{Context, Message};

use crate::{HarnessBinding, TranscriptEvent, TranscriptLine};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessLifecycle {
    Starting,
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, kameo::Reply)]
pub struct HarnessState {
    pub binding: HarnessBinding,
    pub lifecycle: HarnessLifecycle,
    pub transcript_event_count: u64,
}

#[derive(Debug)]
pub struct HarnessActor {
    binding: HarnessBinding,
    lifecycle: HarnessLifecycle,
    transcript_event_count: u64,
}

impl HarnessActor {
    pub fn new(binding: HarnessBinding) -> Self {
        Self {
            binding,
            lifecycle: HarnessLifecycle::Starting,
            transcript_event_count: 0,
        }
    }

    fn state(&self) -> HarnessState {
        HarnessState {
            binding: self.binding.clone(),
            lifecycle: self.lifecycle.clone(),
            transcript_event_count: self.transcript_event_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HarnessActorHandle {
    actor_reference: ActorRef<HarnessActor>,
}

impl HarnessActorHandle {
    pub async fn start(binding: HarnessBinding) -> Self {
        let actor_reference = HarnessActor::spawn(binding);
        actor_reference.wait_for_startup().await;
        Self { actor_reference }
    }

    pub async fn read_state(&self) -> HarnessState {
        self.actor_reference
            .ask(ReadHarnessState)
            .await
            .expect("harness actor mailbox accepts state reads")
    }

    pub async fn set_lifecycle(&self, lifecycle: HarnessLifecycle) -> HarnessState {
        self.actor_reference
            .ask(SetHarnessLifecycle { lifecycle })
            .await
            .expect("harness actor mailbox accepts lifecycle writes")
    }

    pub async fn record_transcript(&self, line: TranscriptLine) -> TranscriptEvent {
        self.actor_reference
            .ask(RecordTranscriptLine { line })
            .await
            .expect("harness actor mailbox accepts transcript writes")
    }

    pub async fn stop(self) {
        self.actor_reference
            .stop_gracefully()
            .await
            .expect("harness actor stops gracefully");
        self.actor_reference.wait_for_shutdown().await;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadHarnessState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetHarnessLifecycle {
    pub lifecycle: HarnessLifecycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordTranscriptLine {
    pub line: TranscriptLine,
}

impl Actor for HarnessActor {
    type Args = HarnessBinding;
    type Error = Infallible;

    async fn on_start(
        binding: Self::Args,
        _actor_reference: ActorRef<Self>,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self::new(binding))
    }
}

impl Message<ReadHarnessState> for HarnessActor {
    type Reply = HarnessState;

    async fn handle(
        &mut self,
        _message: ReadHarnessState,
        _context: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.state()
    }
}

impl Message<SetHarnessLifecycle> for HarnessActor {
    type Reply = HarnessState;

    async fn handle(
        &mut self,
        message: SetHarnessLifecycle,
        _context: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.lifecycle = message.lifecycle;
        self.state()
    }
}

impl Message<RecordTranscriptLine> for HarnessActor {
    type Reply = TranscriptEvent;

    async fn handle(
        &mut self,
        message: RecordTranscriptLine,
        _context: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.transcript_event_count = self.transcript_event_count.saturating_add(1);
        TranscriptEvent::new(self.binding.id().clone(), message.line)
    }
}
