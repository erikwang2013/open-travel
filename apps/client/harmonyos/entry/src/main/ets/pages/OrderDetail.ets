import promptAction from '@ohos.promptAction';
import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class OrderDetailData {
  id: number = 0;
  amount_cents: number = 0;
  status: number = 0;
  created_at: string = '';
  expire_at: string = '';
  title: string = '';
  depart_date: string = '';
  quantity: number = 0;

  constructor(raw: Record<string, Object>) {
    this.id = (raw['id'] as number) ?? 0;
    this.amount_cents = (raw['amount_cents'] as number) ?? 0;
    this.status = (raw['status'] as number) ?? 0;
    this.created_at = (raw['created_at'] as string) ?? '';
    this.expire_at = (raw['expire_at'] as string) ?? '';
    const snap = raw['snapshot'];
    if (snap !== null && snap !== undefined) {
      const s = snap as Record<string, Object>;
      this.title = (s['title'] as string) ?? '';
      this.depart_date = (s['depart_date'] as string) ?? '';
      this.quantity = (s['quantity'] as number) ?? 0;
    }
  }
}

interface OrderDetailParams {
  id?: number;
}

@Entry
@Component
struct OrderDetail {
  @State order: OrderDetailData | null = null;
  @State loading: boolean = true;
  @State failed: boolean = false;
  @State cancelling: boolean = false;
  private orderId: number = 0;

  aboutToAppear(): void {
    const params = router.getParams() as OrderDetailParams;
    this.orderId = params?.id ?? 0;
    this.load();
  }

  async load(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      const raw = await ApiClient.get<Object>(`/api/orders/${this.orderId}`);
      this.order = new OrderDetailData(raw as Record<string, Object>);
    } catch (e) {
      this.failed = true;
    }
    this.loading = false;
  }

  priceText(cents: number): string {
    if (cents % 100 === 0) {
      return '¥' + (cents / 100).toFixed(0);
    }
    return '¥' + (cents / 100).toFixed(2);
  }

  statusText(s: number): Resource {
    if (s === 1) {
      return $r('app.string.order_status_1');
    }
    if (s === 2) {
      return $r('app.string.order_status_2');
    }
    if (s === 3) {
      return $r('app.string.order_status_3');
    }
    if (s === 4) {
      return $r('app.string.order_status_4');
    }
    return $r('app.string.order_status_0');
  }

  statusColor(s: number): string {
    if (s === 1) {
      return '#34C759';
    }
    if (s === 2) {
      return '#0A59F7';
    }
    if (s === 3) {
      return '#8E8E93';
    }
    if (s === 4) {
      return '#999999';
    }
    return '#FF9500';
  }

  canCancel(): boolean {
    return this.order !== null && (this.order.status === 0 || this.order.status === 1);
  }

  doCancel(): void {
    const that = this;
    const buttons: promptAction.Button[] = [
      { text: $r('app.string.cancel_btn'), color: '#999999' },
      { text: $r('app.string.confirm_btn'), color: '#E84026' }
    ];
    promptAction.showDialog({
      title: $r('app.string.cancel_order'),
      message: $r('app.string.cancel_confirm'),
      buttons: buttons
    }, () => {
      that.cancelOrder();
    });
  }

  async cancelOrder(): Promise<void> {
    this.cancelling = true;
    try {
      const emptyBody: Record<string, Object> = {};
      const raw = await ApiClient.post<Object>(`/api/orders/${this.orderId}/cancel`, emptyBody, true);
      this.order = new OrderDetailData(raw as Record<string, Object>);
    } catch (e) {
      this.failed = true;
    }
    this.cancelling = false;
  }

  @Builder
  failedView() {
    Column() {
      Text($r('app.string.load_failed'))
        .fontSize(14)
        .fontColor('#999999')
      Button($r('app.string.retry'), { type: ButtonType.Normal })
        .fontSize(14)
        .fontColor('#0A59F7')
        .backgroundColor(Color.Transparent)
        .margin({ top: 8 })
        .onClick(() => {
          this.load();
        })
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  @Builder
  infoRow(label: Resource, value: string) {
    Row() {
      Text(label)
        .fontSize(14)
        .fontColor('#999999')
        .width(90)
      Text(value)
        .fontSize(14)
        .fontColor('#333333')
        .layoutWeight(1)
    }
    .width('100%')
    .margin({ top: 8 })
  }

  build() {
    Column() {
      Text($r('app.string.order_detail_title'))
        .fontSize(22)
        .fontWeight(FontWeight.Bold)
        .width('100%')
        .padding(16)

      if (this.loading) {
        Column() {
          LoadingProgress().width(48).height(48).color('#0A59F7')
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
      } else if (this.failed || !this.order) {
        this.failedView()
      } else {
        Scroll() {
          Column() {
            Row() {
              Text(this.order.title.length > 0 ? this.order.title : ('#' + this.order.id.toString()))
                .fontSize(18)
                .fontWeight(FontWeight.Bold)
                .layoutWeight(1)
              Text(this.statusText(this.order.status))
                .fontSize(11)
                .fontColor('#FFFFFF')
                .padding({ left: 8, right: 8, top: 3, bottom: 3 })
                .borderRadius(8)
                .backgroundColor(this.statusColor(this.order.status))
            }
            .width('100%')

            Text($r('app.string.order_snapshot'))
              .fontSize(15)
              .fontWeight(FontWeight.Bold)
              .width('100%')
              .margin({ top: 16 })

            Column() {
              this.infoRow($r('app.string.order_no'), '#' + this.order.id.toString())
              this.infoRow($r('app.string.order_amount'), this.priceText(this.order.amount_cents))
              if (this.order.depart_date.length > 0) {
                this.infoRow($r('app.string.depart_date'), this.order.depart_date)
              }
              if (this.order.quantity > 0) {
                this.infoRow($r('app.string.quantity'), this.order.quantity.toString())
              }
              this.infoRow($r('app.string.order_created'), this.order.created_at)
              if (this.order.expire_at.length > 0) {
                this.infoRow($r('app.string.expire_at'), this.order.expire_at)
              }
            }
            .alignItems(HorizontalAlign.Start)
            .width('100%')
            .padding(16)
            .backgroundColor('#FFFFFF')
            .borderRadius(12)
            .margin({ top: 8 })

            if (this.canCancel()) {
              Button(this.cancelling ? $r('app.string.cancelling') : $r('app.string.cancel_order'))
                .width('100%')
                .height(44)
                .backgroundColor('#E84026')
                .margin({ top: 16, bottom: 24 })
                .enabled(!this.cancelling)
                .onClick(() => {
                  this.doCancel();
                })
            }
          }
          .alignItems(HorizontalAlign.Start)
          .width('100%')
          .padding(16)
        }
        .layoutWeight(1)
        .width('100%')
        .backgroundColor('#F5F5F5')
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }
}
