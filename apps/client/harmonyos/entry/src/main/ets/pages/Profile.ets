@Entry
@Component
struct Profile {
  @State username: string = 'traveler@opentravel.com';

  build() {
    Column() {
      Text('OT')
        .fontSize(28)
        .fontColor('#FFFFFF')
        .textAlign(TextAlign.Center)
        .width(72)
        .height(72)
        .borderRadius(36)
        .backgroundColor('#0A59F7')
        .margin({ top: 48 })

      Text($r('app.string.app_name'))
        .fontSize(20)
        .fontWeight(FontWeight.Bold)
        .margin({ top: 16 })

      Text(this.username)
        .fontSize(14)
        .fontColor('#666666')
        .margin({ top: 8 })

      Row() {
        Text($r('app.string.language_label'))
          .fontSize(16)
        Blank()
        Text('中文 / English')
          .fontSize(14)
          .fontColor('#999999')
      }
      .width('90%')
      .padding(16)
      .backgroundColor('#FFFFFF')
      .borderRadius(10)
      .margin({ top: 32 })

      Blank()

      Button($r('app.string.logout'))
        .width('90%')
        .height(44)
        .backgroundColor('#E84026')
        .margin({ bottom: 24 })
        .onClick(() => {
          // TODO: 接入 user 服务 /api/v1/user/profile（8001）与登出
        })
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }
}
