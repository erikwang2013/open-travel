import router from '@ohos.router';

@Entry
@Component
struct Index {
  @State currentIndex: number = 0;
  private hotDestinations: string[] = ['三亚', '北京', '张家界', '桂林', '杭州', '成都'];

  @Builder
  tabBuilder(title: Resource, index: number) {
    Column() {
      Text(title)
        .fontSize(13)
        .fontColor(this.currentIndex === index ? '#0A59F7' : '#666666')
        .fontWeight(this.currentIndex === index ? FontWeight.Medium : FontWeight.Normal)
    }
    .width('100%')
    .height('100%')
    .justifyContent(FlexAlign.Center)
  }

  @Builder
  homeTab() {
    Column() {
      Row() {
        Text($r('app.string.hot_destinations'))
          .fontSize(22)
          .fontWeight(FontWeight.Bold)
        Blank()
        Button($r('app.string.view_all'), { type: ButtonType.Text })
          .fontSize(14)
          .fontColor('#0A59F7')
          .onClick(() => {
            this.openDestinationList();
          })
      }
      .width('100%')
      .padding({ left: 16, right: 16, top: 16, bottom: 8 })

      List({ space: 12 }) {
        ForEach(this.hotDestinations, (item: string) => {
          ListItem() {
            Row() {
              Text(item)
                .fontSize(16)
                .layoutWeight(1)
              Text('›')
                .fontSize(20)
                .fontColor('#999999')
            }
            .width('100%')
            .padding(16)
            .backgroundColor('#FFFFFF')
            .borderRadius(10)
            .onClick(() => {
              this.openDestinationList();
            })
          }
        }, (item: string) => item)
      }
      .width('100%')
      .layoutWeight(1)
      .padding({ left: 16, right: 16, top: 8 })
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }

  @Builder
  searchTab() {
    Column() {
      TextInput({ placeholder: $r('app.string.search_hint') })
        .width('90%')
        .margin({ top: 24 })
      Blank()
      Text($r('app.string.search_placeholder'))
        .fontSize(14)
        .fontColor('#999999')
      Blank()
    }
    .width('100%')
    .height('100%')
  }

  @Builder
  bookingTab() {
    Column() {
      Blank()
      Text($r('app.string.booking_placeholder'))
        .fontSize(14)
        .fontColor('#999999')
      Blank()
    }
    .width('100%')
    .height('100%')
  }

  @Builder
  profileTab() {
    Column() {
      Blank()
      Text('OT')
        .fontSize(28)
        .fontColor('#FFFFFF')
        .textAlign(TextAlign.Center)
        .width(72)
        .height(72)
        .borderRadius(36)
        .backgroundColor('#0A59F7')
      Text($r('app.string.app_name'))
        .fontSize(18)
        .fontWeight(FontWeight.Bold)
        .margin({ top: 12 })
      Text($r('app.string.profile_subtitle'))
        .fontSize(13)
        .fontColor('#999999')
        .margin({ top: 4 })
      Button($r('app.string.profile_enter'), { type: ButtonType.Text })
        .fontSize(14)
        .fontColor('#0A59F7')
        .margin({ top: 16 })
        .onClick(() => {
          router.pushUrl({ url: 'pages/Profile' });
        })
      Blank()
    }
    .width('100%')
    .height('100%')
  }

  openDestinationList(): void {
    router.pushUrl({ url: 'pages/DestinationList' });
  }

  build() {
    Tabs({ barPosition: BarPosition.End, index: this.currentIndex }) {
      TabContent() {
        this.homeTab()
      }
      .tabBar(this.tabBuilder($r('app.string.tab_home'), 0))

      TabContent() {
        this.searchTab()
      }
      .tabBar(this.tabBuilder($r('app.string.tab_search'), 1))

      TabContent() {
        this.bookingTab()
      }
      .tabBar(this.tabBuilder($r('app.string.tab_booking'), 2))

      TabContent() {
        this.profileTab()
      }
      .tabBar(this.tabBuilder($r('app.string.tab_profile'), 3))
    }
    .onChange((index: number) => {
      this.currentIndex = index;
    })
    .width('100%')
    .height('100%')
    .backgroundColor('#FFFFFF')
  }
}
