@Entry
@Component
struct DestinationDetail {
  build() {
    Column() {
      Text($r('app.string.destination_detail_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      Column() {
        Text($r('app.string.detail_placeholder'))
          .fontSize(14)
          .fontColor('#999999')
          .textAlign(TextAlign.Center)
      }
      .width('90%')
      .padding(24)
      .backgroundColor('#FFFFFF')
      .borderRadius(12)
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }
}
