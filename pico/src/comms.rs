use bt_hci::controller::ExternalController;
use cyw43::bluetooth::BtDriver;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::{join::join, select::select};
use embassy_rp::{
    Peripheral,
    dma::Channel,
    gpio::{Level, Output},
    interrupt::typelevel::Binding,
    peripherals::{DMA_CH6, PIO1},
    pio::{Instance as PioInstance, InterruptHandler, Pio, PioPin},
};
use embassy_time::{Duration, Timer, with_timeout};
use static_cell::StaticCell;
use trouble_host::prelude::*;
use trouble_host::{Address, Controller, HostResources};

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC

pub struct Comms<const N: usize>;

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO1, 0, DMA_CH6>>,
) -> ! {
    runner.run().await
}

impl<const N: usize> Comms<N> {
    pub async fn new<DIO: PioPin, CLK: PioPin>(
        spawner: Spawner,
        pwr: impl Peripheral<P = impl PioPin> + 'static,
        cs: impl Peripheral<P = impl PioPin> + 'static,
        dio: DIO,
        clk: CLK,
        channel: impl Peripheral<P = DMA_CH6> + 'static,
        mut pio: Pio<'static, PIO1>,
    ) {
        info!("setting up stuff");
        let (fw, clm, btfw) = {
            // IMPORTANT
            //
            // Download and make sure these files from https://github.com/embassy-rs/embassy/tree/main/cyw43-firmware
            // are available in `./examples/rp-pico-2-w`. (should be automatic)
            //
            // IMPORTANT
            let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
            let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");
            let btfw = include_bytes!("../cyw43-firmware/43439A0_btfw.bin");
            (fw, clm, btfw)
        };

        let pwr = Output::new(pwr, Level::Low);
        let cs = Output::new(cs, Level::High);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            RM2_CLOCK_DIVIDER,
            pio.irq0,
            cs,
            dio,
            clk,
            channel,
        );

        info!("Set up pio spi");

        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());
        let (_net_device, bt_device, mut control, runner) =
            cyw43::new_with_bluetooth(state, pwr, spi, fw, btfw).await;
        info!("Spawning the cyw43 task");
        unwrap!(spawner.spawn(cyw43_task(runner)));
        info!("Initing the controller");
        control.init(clm).await;

        let controller: ExternalController<_, 10> = ExternalController::new(bt_device);

        unwrap!(spawner.spawn(run_task(controller)));
    }
}

#[embassy_executor::task]
async fn run_task(controller: ExternalController<BtDriver<'static>, 10>) {
    run(controller).await;
}

pub async fn run<C>(controller: C)
where
    C: Controller,
{
    // Hardcoded peripheral address
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);

    let mut resources: HostResources<CONNECTIONS_MAX, L2CAP_CHANNELS_MAX, 251, 16> =
        HostResources::new();
    info!("Create resources");
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host {
        mut peripheral,
        mut runner,
        ..
    } = stack.build();

    let mut adv_data = [0; 31];
    let adv_data_len = AdStructure::encode_slice(
        &[AdStructure::Flags(
            LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED,
        )],
        &mut adv_data[..],
    )
    .unwrap();

    let mut scan_data = [0; 31];
    let scan_data_len = AdStructure::encode_slice(
        &[AdStructure::CompleteLocalName(b"Sab's piccy thing")],
        &mut scan_data[..],
    )
    .unwrap();

    info!("Hi");

    let _ = join(runner.run(), async {
        'connect_loop: loop {
            info!("Creating advertiser");
            let advertiser = peripheral
                .advertise(
                    &Default::default(),
                    Advertisement::ConnectableScannableUndirected {
                        adv_data: &adv_data[..adv_data_len],
                        scan_data: &scan_data[..scan_data_len],
                    },
                )
                .await
                .unwrap();
            info!("Waiting for connection");
            let conn = advertiser.accept().await.unwrap();

            info!("Connection established");

            let config = L2capChannelConfig {
                mtu: 256,
                ..Default::default()
            };

            info!("Creating l2cap channel");

            let mut ch1 = match with_timeout(
                Duration::from_millis(1000),
                L2capChannel::accept(&stack, &conn, &[10], &config),
            )
            .await
            {
                Ok(ch_res) => ch_res.unwrap(),
                Err(_) => continue,
            };

            info!("L2CAP channel accepted");

            // Size of payload we're expecting
            const PAYLOAD_LEN: usize = 27;
            let mut rx = [0; PAYLOAD_LEN];
            for i in 0..10 {
                match ch1.receive(&stack, &mut rx).await {
                    Ok(len) => {
                        info!("Received a payload: {}", rx[..len])
                    }
                    Err(_) => {
                        error!("Got bluetooth error, closing");
                        ch1.disconnect();
                        conn.disconnect();
                        continue 'connect_loop;
                    }
                }
            }

            info!("L2CAP data received, echoing");
            Timer::after(Duration::from_secs(1)).await;
            for i in 0..10 {
                let tx = [i; PAYLOAD_LEN];
                ch1.send::<C, 100>(&stack, &tx).await.unwrap();
            }
            ch1.disconnect();
            conn.disconnect();
            info!("L2CAP data echoed");
        }
    })
    .await;
}
