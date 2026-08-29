import promptAction from '@ohos.promptAction';
import router from '@ohos.router';
import common from '@ohos.app.ability.common';
import Want from '@ohos.app.ability.Want';
import { ApiClient } from '../services/ApiClient';

class ChannelItem {
  channel_code: string = '';
  name: string = '';
  type: string = '';
  enabled: boolean = false;

  constructor(raw: Record<string, Object>) {
    this.channel_code = (raw['channel_code'] as string) ?? '';
    this.type = (raw['type'] as string) ?? '';
    this.enabled = (raw['enabled'] as boolean) ?? false;
    this.name = PaymentPage.channelName(raw);
  }
}

interface PaymentParams {
  id?: number;
}

@Entry
@Component
struct PaymentPage {
  @State channels: ChannelItem[] = [];
  @State loading: boolean = true;
  @State failed: boolean = false;
  @State selected: number = -1;
  @State creating: boolean = false;
  @State createFailed: boolean = false;
  @State txnNo: string = '';
  @State checkoutUrl: string = '';
  @State amountCents: number = 0;
  @State paid: boolean = false;
  @State paidResult: boolean = false;
  @State createError: string = '';
  private orderId: number = 0;
  private timer: number = -1;

  static channelName(raw: Record<string, Object>): string {
    const name = raw['name'];
    if (typeof name === 'string' && (name as string).length > 0) {
      return name as string;
    }
    if (name !== null && name !== undefined) {
      const m = name as Record<string, Object>;
      const lang = ApiClient.lang();
      const v = m[lang];
      if (v !== null && v !== undefined && (v as string).length > 0) {
        return v as string;
      }
      const ev = m['en'];
      if (ev !== null && ev !== undefined && (ev as string).length > 0) {
        return ev as string;
      }
    }
    return (raw['channel_code'] as string) ?? '';
  }

  aboutToAppear(): void {
    const params = router.getParams() as PaymentParams;
    this.orderId = params?.id ?? 0;
    if (!ApiClient.isLoggedIn()) {
      router.replaceUrl({ url: 'pages/LoginPage' });
      return;
    }
    this.loadChannels();
  }

  aboutToDisappear(): void {
    if (this.timer >= 0) {
      clearInterval(this.timer);
      this.timer = -1;
    }
  }

  async loadChannels(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      const data = await ApiClient.get<Object>(`/api/payments/channels?lang=${ApiClient.lang()}`);
      const items = (data as Record<string, Object>)['items'];
      this.channels = [];
      if (items instanceof Array) {
        this.channels = (items as Object[]).map((it: Object) => new ChannelItem(it as Record<string, Object>));
      }
      if (this.channels.length > 0) {
        this.selected = 0;
      }
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

  async createPayment(): Promise<void> {
    this.createFailed = false;
    this.createError = '';
    if (this.selected < 0) {
      return;
    }
    this.creating = true;
    try {
      const body: Record<string, Object> = {
        'order_id': this.orderId,
        'channel_code': this.channels[this.selected].channel_code
      };
      const data = await ApiClient.post<Record<string, Object>>('/api/payments', body, true);
      this.txnNo = (data['txn_no'] as string) ?? '';
      this.checkoutUrl = (data['checkout_url'] as string) ?? '';
      this.amountCents = (data['amount_cents'] as number) ?? 0;
      this.startPoll();
    } catch (e) {
      this.createFailed = true;
      const msg = (e as Error).message;
      this.createError = msg.length > 0 ? msg : '';
    }
    this.creating = false;
  }

  startPoll(): void {
    if (this.timer >= 0) {
      clearInterval(this.timer);
    }
    const that = this;
    this.timer = setInterval(() => {
      that.pollOnce();
    }, 3000);
  }

  async pollOnce(): Promise<void> {
    try {
      const raw = await ApiClient.get<Object>(`/api/orders/${this.orderId}`, true);
      const status = (raw as Record<string, Object>)['status'] as number;
      if (status !== undefined && status !== null && status !== 0) {
        if (this.timer >= 0) {
          clearInterval(this.timer);
          this.timer = -1;
        }
        this.paid = true;
        this.paidResult = status === 1 || status === 2 || status === 3;
      }
    } catch (e) {
      // transient failure: keep polling
    }
  }

  openCheckout(): void {
    if (this.checkoutUrl.length === 0) {
      return;
    }
    const ctx = getContext(this) as common.UIAbilityContext;
    const want: Want = { action: 'ohos.want.action.viewData', uri: this.checkoutUrl };
    ctx.startAbility(want).catch((e: Error) => {
      promptAction.showToast({ message: e.message });
    });
  }

  done(): void {
    router.back();
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
          this.loadChannels();
        })
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  @Builder
  channelRow(ch: ChannelItem, index: number) {
    Row() {
      Text(ch.name)
        .fontSize(16)
        .layoutWeight(1)
      Text(this.selected === index ? '●' : '○')
        .fontSize(18)
        .fontColor(this.selected === index ? '#0A59F7' : '#BBBBBB')
    }
    .width('100%')
    .padding(16)
    .backgroundColor('#FFFFFF')
    .borderRadius(12)
    .margin({ top: 10 })
    .onClick(() => {
      this.selected = index;
    })
  }

  build() {
    Column() {
      Text($r('app.string.pay_title'))
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
      } else if (this.failed) {
        this.failedView()
      } else if (this.paid) {
        Column() {
          Text(this.paidResult ? $r('app.string.pay_success') : $r('app.string.pay_failed'))
            .fontSize(20)
            .fontWeight(FontWeight.Bold)
            .fontColor(this.paidResult ? '#34C759' : '#E84026')
          Button($r('app.string.confirm_btn'))
            .width('60%')
            .height(44)
            .backgroundColor('#0A59F7')
            .margin({ top: 24 })
            .onClick(() => {
              this.done();
            })
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
      } else if (this.txnNo.length === 0) {
        Scroll() {
          Column() {
            Text($r('app.string.pay_channel_hint'))
              .fontSize(15)
              .fontWeight(FontWeight.Bold)
              .width('100%')
            ForEach(this.channels, (ch: ChannelItem, index: number) => {
              this.channelRow(ch, index)
            }, (ch: ChannelItem) => ch.channel_code)

            if (this.createFailed) {
              Text(this.createError.length > 0 ? this.createError : $r('app.string.book_failed'))
                .fontSize(13)
                .fontColor('#E84026')
                .width('100%')
                .margin({ top: 12 })
            }

            Button(this.creating ? $r('app.string.paying') : $r('app.string.pay_confirm'))
              .width('100%')
              .height(44)
              .backgroundColor('#0A59F7')
              .margin({ top: 16, bottom: 24 })
              .enabled(!this.creating && this.selected >= 0)
              .onClick(() => {
                this.createPayment();
              })
          }
          .alignItems(HorizontalAlign.Start)
          .width('100%')
          .padding(16)
        }
        .layoutWeight(1)
        .width('100%')
        .backgroundColor('#F5F5F5')
      } else {
        Column() {
          Text(this.priceText(this.amountCents))
            .fontSize(28)
            .fontWeight(FontWeight.Bold)
            .fontColor('#E84026')
          Text($r('app.string.checkout_hint'))
            .fontSize(13)
            .fontColor('#999999')
            .margin({ top: 8 })
          Text(this.checkoutUrl)
            .fontSize(12)
            .fontColor('#666666')
            .width('100%')
            .textAlign(TextAlign.Center)
            .margin({ top: 8 })

          Button($r('app.string.open_checkout'))
            .width('80%')
            .height(44)
            .backgroundColor('#0A59F7')
            .margin({ top: 32 })
            .onClick(() => {
              this.openCheckout();
            })
          Button($r('app.string.pay_done'), { type: ButtonType.Normal })
            .width('80%')
            .height(44)
            .fontColor('#0A59F7')
            .backgroundColor(Color.Transparent)
            .margin({ top: 8 })
            .onClick(() => {
              this.pollOnce();
            })

          Row() {
            LoadingProgress().width(20).height(20).color('#0A59F7')
            Text($r('app.string.pay_pending'))
              .fontSize(13)
              .fontColor('#999999')
              .margin({ left: 8 })
          }
          .margin({ top: 24 })
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
        .padding(24)
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F5F5F5')
  }
}
