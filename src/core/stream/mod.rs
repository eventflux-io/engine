// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod input;
pub mod junction_factory;
pub mod optimized_stream_junction;
pub mod output;
pub mod stream_initializer;
pub mod stream_junction;

pub use self::input::source::{timer_source::TimerSource, Source};
pub use self::input::{InputHandler, InputManager};
pub use self::junction_factory::{
    BenchmarkResult, JunctionBenchmark, JunctionConfig, JunctionType, PerformanceLevel,
    StreamJunctionFactory,
};
pub use self::optimized_stream_junction::{
    JunctionPerformanceMetrics, OptimizedPublisher, OptimizedStreamJunction,
};
pub use self::output::{LogSink, Sink, StreamCallback};
pub use self::stream_initializer::{
    initialize_stream, InitializedSink, InitializedSource, InitializedStream,
};
pub use self::stream_junction::{
    OnErrorAction, Publisher, Receiver as StreamJunctionReceiver, StreamJunction,
};
