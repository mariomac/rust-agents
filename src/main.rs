use crate::tracers::proc::Process;

mod export;
mod graph;
mod tracers;
mod transform;

#[tokio::main]
async fn main() {
    graph! {
        extract {
            procs:       tracers::proc::trace_processes,
        }
        transform {
            proc_renamer: transform::proc_renamer,
        }
        load {
            otel_export: export::otel::exporter,
        }

        procs -> proc_renamer -> otel_export,
    };
}
