import http from '@ohos.net.http';
import i18n from '@ohos.i18n';

interface ApiEnvelope<T> {
  code: number;
  message: string;
  data: T;
}

export class ApiClient {
  static readonly BASE_URL: string = 'http://localhost:8082';
  static readonly API_VERSION: string = 'v1';

  static lang(): string {
    const sys = i18n.System.getSystemLanguage();
    if (sys.startsWith('zh')) {
      return 'zh';
    }
    return 'en';
  }

  static async get<T>(path: string): Promise<T> {
    const client = http.createHttp();
    try {
      const resp = await client.request(ApiClient.BASE_URL + path, {
        method: http.RequestMethod.GET,
        connectTimeout: 10000,
        readTimeout: 10000,
        header: {
          'Content-Type': 'application/json',
          'X-Api-Version': ApiClient.API_VERSION
        }
      });
      const code: number = resp.responseCode;
      if (code < 200 || code >= 300) {
        throw new Error('HTTP ' + code);
      }
      const env = JSON.parse(resp.result as string) as ApiEnvelope<T>;
      if (env.code !== 0) {
        throw new Error(env.message || ('API ' + env.code));
      }
      return env.data;
    } finally {
      client.destroy();
    }
  }
}
