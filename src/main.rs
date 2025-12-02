mod export;
mod tracers;
mod transform;

use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

macro_rules! graph {
    {
        extract {
            $($enode_name:ident : $enode_func:path,)*
        }
        transform {
            $($tnode_name:ident : $tnode_func:path,)*
        }
        load {
            $($lnode_name:ident : $lnode_func:path,)*
        }
        $($src_name:ident -> $dst_name:path,)*
    } => {
        fn check_enode_signature<T: Send + 'static>(
            _f: impl Fn(CancellationToken, tokio::sync::mpsc::Sender<T>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        ) {}

        let ct = CancellationToken::new();
        // Validate extract node signatures at compile time
        $(
            const _: fn() = || {
                // This will fail to compile if $enode_func doesn't match the expected signature
                check_enode_signature(|ct, out| Box::pin($enode_func(ct, out)));
            };
        )*
        // TODO: validate tnode and lnode signatures

        let mut channels = HashMap::new();
        $({
            // TODO: optimize (e.g. use only MPSC when multiple submitters are detected)
            let (rx, tx) = tokio::sync::mpsc::channel(100);
            channels.insert($tnode_name, tx);
        })*
        $({
            // TODO: optimize (e.g. use only MPSC when multiple submitters are detected)
            let (rx, tx) = tokio::sync::mpsc::channel(100);
            channels.insert($lnode_name, tx);
        })*



        // let ct = CancellationToken::new();
    };
}

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

    let ct = CancellationToken::new();

    let (procs_output, transform_input) = tokio::sync::mpsc::channel(100);
    let (transform_output, export_input) = tokio::sync::mpsc::channel(100);

    tokio::join!(
        tracers::proc::trace_processes(ct.clone(), procs_output),
        transform::proc_renamer(ct.clone(), transform_input, transform_output),
        export::otel::exporter(ct.clone(), export_input),
    );
}
