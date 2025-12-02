mod export;
mod tracers;
mod transform;
mod graph;


use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;


#[tokio::main]
async fn main() {
    let a = 33;

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
        procs -> proc_renamer,
        proc_renamer -> otel_export,
    };

    // let ct = CancellationToken::new();
    // 
    // let (procs_output, transform_input) = tokio::sync::mpsc::channel(100);
    // let (transform_output, export_input) = tokio::sync::mpsc::channel(100);
    // 
    // tokio::join!(
    //     tracers::proc::trace_processes(ct.clone(), procs_output),
    //     transform::proc_renamer(ct.clone(), transform_input, transform_output),
    //     export::otel::exporter(ct.clone(), export_input),
    // );
}
