
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
            // proc_renamer: transform::proc_renamer,
        }
        load {
            export: export::prom::metrics_exporter,
        }

        procs -> export,
        // procs -> proc_renamer -> otel_export,
    };
}
