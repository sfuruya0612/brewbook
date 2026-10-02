//! `records::RecordServices` の単体テスト (0041 が使う依存の束)。

mod support;

use std::rc::Rc;

use dioxus::prelude::*;

use brew_book_frontend::records::values::LocalDateTime;
use brew_book_frontend::records::{PickedPhoto, RecordServices, MAX_PHOTO_LONG_SIDE};

use support::{block_on, client, FakeClock, FakeImageConverter, FakePhotoPicker};

/// 画面の prop として受け取れることを型で確認する (Dioxus の prop は Clone + PartialEq を要求する)。
#[component]
fn Screen(services: RecordServices) -> Element {
    let _ = services;
    rsx! {
        div {}
    }
}

/// 偽の依存を束ねた [`RecordServices`] を作る。
fn services() -> (RecordServices, Rc<dyn brew_book_frontend::records::Clock>) {
    let (api, _) = client(vec![]);
    let clock: Rc<dyn brew_book_frontend::records::Clock> = Rc::new(FakeClock {
        now: LocalDateTime::new(2026, 10, 2, 9, 30),
        utc_offset_minutes: 540,
    });
    let picker = Rc::new(FakePhotoPicker {
        photo: Some(PickedPhoto {
            name: "photo.jpg".to_string(),
            bytes: vec![1, 2, 3],
        }),
    });
    let converter = Rc::new(FakeImageConverter);
    (
        RecordServices::new(api, clock.clone(), picker, converter),
        clock,
    )
}

#[test]
fn the_services_bundle_the_record_dependencies() {
    let (services, clock) = services();

    // 端末の時計とタイムゾーン (FR-18)。
    assert_eq!(services.clock.now(), LocalDateTime::new(2026, 10, 2, 9, 30));
    assert_eq!(services.clock.utc_offset_minutes(), 540);

    // 写真の選択と変換 (FR-10)。
    let photo = block_on(services.photo_picker.pick_photo())
        .expect("the picker must not fail")
        .expect("a photo must be selected");
    assert_eq!(photo.name, "photo.jpg");
    let image = block_on(
        services
            .image_converter
            .convert_jpeg(photo.bytes, MAX_PHOTO_LONG_SIDE),
    )
    .expect("the converter must not fail");
    assert_eq!(image.size(), 3);

    // 依存は同じ実体を指す (画面へ配っても増えない)。
    assert!(Rc::ptr_eq(&services.clock, &clock));
}

#[test]
fn the_services_can_be_cloned_and_compared_for_a_screen() {
    let (services, _) = services();
    let copy = services.clone();

    assert!(copy == services);
    // 依存を指す実体は共有される。
    assert!(Rc::ptr_eq(&copy.clock, &services.clock));
}

#[test]
fn a_screen_prop_can_take_the_services() {
    let (services, _) = services();

    let props = ScreenProps {
        services: services.clone(),
    };

    assert!(props.services == services);
}
