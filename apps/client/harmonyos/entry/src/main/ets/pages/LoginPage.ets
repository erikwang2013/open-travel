import router from '@ohos.router';
import { ApiClient } from '../services/ApiClient';

class LoginResult {
  token: string = '';
  user_id: number = 0;
  email: string = '';
}

@Entry
@Component
struct LoginPage {
  @State email: string = '';
  @State password: string = '';
  @State loading: boolean = false;
  @State failed: boolean = false;

  async doLogin(): Promise<void> {
    if (this.email.length === 0 || this.password.length === 0) {
      this.failed = true;
      return;
    }
    this.loading = true;
    this.failed = false;
    try {
      const body: Record<string, Object> = { 'email': this.email, 'password': this.password };
      const res = await ApiClient.post<LoginResult>('/api/user/login', body);
      if (res.token.length === 0) {
        throw new Error('empty token');
      }
      AppStorage.SetOrCreate('auth_token', res.token);
      AppStorage.SetOrCreate('user_email', res.email);
      router.back();
    } catch (e) {
      this.failed = true;
    }
    this.loading = false;
  }

  build() {
    Column() {
      Text($r('app.string.login_title'))
        .fontSize(24)
        .fontWeight(FontWeight.Bold)
        .margin({ top: 64, bottom: 32 })

      TextInput({ placeholder: $r('app.string.login_email_hint'), text: this.email })
        .width('88%')
        .height(46)
        .margin({ bottom: 14 })
        .onChange((v: string) => {
          this.email = v;
        })

      TextInput({ placeholder: $r('app.string.login_password_hint'), text: this.password })
        .type(InputType.Password)
        .width('88%')
        .height(46)
        .margin({ bottom: 24 })
        .onChange((v: string) => {
          this.password = v;
        })

      if (this.failed) {
        Text($r('app.string.login_failed'))
          .fontSize(13)
          .fontColor('#E84026')
          .margin({ bottom: 12 })
      }

      Button(this.loading ? $r('app.string.logging_in') : $r('app.string.login_button'))
        .width('88%')
        .height(46)
        .backgroundColor('#0A59F7')
        .onClick(() => {
          this.doLogin();
        })
    }
    .width('100%')
    .height('100%')
    .alignItems(HorizontalAlign.Center)
    .backgroundColor('#F5F5F5')
  }
}
