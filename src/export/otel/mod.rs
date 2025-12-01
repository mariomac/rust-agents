use crate::tracers::proc::Process;
use tokio_util::sync::CancellationToken;

pub struct Config {}

pub async fn exporter(ct: CancellationToken, mut input: tokio::sync::mpsc::Receiver<Vec<Process>>) {
    let export = opentelemetry_stdout::MetricExporter::default();

    loop {
        tokio::select!(
            _ = ct.cancelled() => {
                println!("exporter cancelled");
                return;
            }
            Some(stuff) = input.recv() => {
                println!("received {} processes", stuff.len());
                stuff.iter().take(3).for_each(|proc| println!(" > {:?}", proc.name));
                println!("...");
            },
        );
    }
    /*
    let provider = Arc::new(
        SdkMeterProvider::builder()
            .with_periodic_exporter(export)
            .build(),
    );

    let meter = Arc::new(provider.meter("process_metrics"));
    Box::new(move || -> InstanceResult {
        let meter = meter.clone();
        let runner = |ct:CancellationToken| async move {
            let count = meter.u64_counter("metricaca").build();
            loop {
                count.add(1, &[KeyValue::new("triki", "traka")]);
            }
        };
        Ok(swarm_run!(runner))
    })*/
}
