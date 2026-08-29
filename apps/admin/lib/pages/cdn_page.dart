import 'package:flutter/material.dart';

import '../api.dart';

class CdnProvider {
  CdnProvider({
    required this.providerCode,
    required this.name,
    required this.enabled,
    this.bucket = '',
    this.region = '',
    this.domain = '',
    this.endpoint = '',
  });

  CdnProvider.fromJson(Map<String, dynamic> j)
      : providerCode = (j['provider_code'] ?? '') as String,
        name = (j['name'] ?? '') as String,
        enabled = j['enabled'] == true,
        bucket = (j['bucket'] ?? '') as String,
        region = (j['region'] ?? '') as String,
        domain = (j['domain'] ?? '') as String,
        endpoint = (j['endpoint'] ?? '') as String;

  final String providerCode;
  final String name;
  final bool enabled;
  final String bucket;
  final String region;
  final String domain;
  final String endpoint;

  CdnProvider copyWith({bool? enabled}) => CdnProvider(
        providerCode: providerCode,
        name: name,
        enabled: enabled ?? this.enabled,
        bucket: bucket,
        region: region,
        domain: domain,
        endpoint: endpoint,
      );
}

class CdnPage extends StatefulWidget {
  const CdnPage({super.key});

  @override
  State<CdnPage> createState() => _CdnPageState();
}

class _CdnPageState extends State<CdnPage> {
  List<CdnProvider> _list = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final data = await Api.fetchCdnProviders();
      final items = data is List ? data : (data['items'] as List);
      setState(() {
        _list = items
            .map((e) => CdnProvider.fromJson(e as Map<String, dynamic>))
            .toList();
        _loading = false;
      });
    } on ApiException catch (e) {
      setState(() {
        _error = e.message;
        _loading = false;
      });
    } catch (e) {
      setState(() {
        _error = '网络错误：$e';
        _loading = false;
      });
    }
  }

  void _showSnack(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
  }

  Future<void> _toggle(CdnProvider p, bool on) async {
    final idx = _list.indexWhere((e) => e.providerCode == p.providerCode);
    setState(() => _list[idx] = p.copyWith(enabled: on));
    try {
      await Api.updateCdnProviderStatus(p.providerCode, on);
      _showSnack('${p.name} 已${on ? '启用' : '禁用'}');
    } catch (e) {
      setState(() => _list[idx] = p);
      _showSnack('操作失败：$e');
    }
  }

  Future<void> _showConfigSheet(CdnProvider p) async {
    final bucket = TextEditingController(text: p.bucket);
    final region = TextEditingController(text: p.region);
    final domain = TextEditingController(text: p.domain);
    final endpoint = TextEditingController(text: p.endpoint);
    final saved = await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => Padding(
        padding: EdgeInsets.only(
            left: 16,
            right: 16,
            top: 16,
            bottom: MediaQuery.of(ctx).viewInsets.bottom + 16),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('配置 ${p.name}', style: Theme.of(ctx).textTheme.titleMedium),
              const SizedBox(height: 16),
              TextField(
                  controller: bucket,
                  decoration: const InputDecoration(
                      labelText: 'Bucket',
                      border: OutlineInputBorder())),
              const SizedBox(height: 12),
              TextField(
                  controller: region,
                  decoration: const InputDecoration(
                      labelText: '区域 Region',
                      border: OutlineInputBorder())),
              const SizedBox(height: 12),
              TextField(
                  controller: domain,
                  decoration: const InputDecoration(
                      labelText: '加速域名 Domain',
                      border: OutlineInputBorder())),
              const SizedBox(height: 12),
              TextField(
                  controller: endpoint,
                  decoration: const InputDecoration(
                      labelText: 'Endpoint',
                      border: OutlineInputBorder())),
              const SizedBox(height: 16),
              SizedBox(
                width: double.infinity,
                child: FilledButton(
                  onPressed: () => Navigator.pop(ctx, true),
                  child: const Text('保存'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
    if (saved != true) return;
    try {
      await Api.saveCdnProvider(p.providerCode, {
        'bucket': bucket.text.trim(),
        'region': region.text.trim(),
        'domain': domain.text.trim(),
        'endpoint': endpoint.text.trim(),
      });
      _showSnack('${p.name} 配置已保存');
      _load();
    } catch (e) {
      _showSnack('保存失败：$e');
    }
  }

  Future<void> _showPlan(CdnProvider p) async {
    try {
      final data = await Api.getCdnPlan(p.providerCode);
      final cmds = data['commands'] ?? data;
      final text =
          cmds is List ? cmds.map((e) => e.toString()).join('\n') : cmds.toString();
      if (!mounted) return;
      showDialog<void>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: Text('${p.name} 命令预览'),
          content: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                SelectableText(text,
                    style: const TextStyle(
                        fontFamily: 'monospace', fontSize: 13)),
                const SizedBox(height: 12),
                Text('提示：真实执行需在部署机配置云 CLI 凭据。',
                    style: Theme.of(ctx).textTheme.bodySmall),
              ],
            ),
          ),
          actions: [
            TextButton(
                onPressed: () => Navigator.pop(ctx), child: const Text('关闭')),
          ],
        ),
      );
    } catch (e) {
      _showSnack('生成命令失败：$e');
    }
  }

  String _summary(CdnProvider p) {
    final bucket = p.bucket.isEmpty ? '未配置 Bucket' : p.bucket;
    final region = p.region.isEmpty ? '-' : p.region;
    final domain = p.domain.isEmpty ? '' : '，域名：${p.domain}';
    return 'Bucket：$bucket，区域：$region$domain';
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
          child: Row(
            children: [
              Text('CDN 云服务商管理',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(width: 12),
              Expanded(
                child: Text('开关即时生效；配置与生成命令需后端接口支持',
                    style: Theme.of(context).textTheme.bodySmall),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: _loading
              ? const Center(child: CircularProgressIndicator())
              : _error != null
                  ? Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(_error!),
                          const SizedBox(height: 8),
                          FilledButton(
                              onPressed: _load, child: const Text('重试')),
                        ],
                      ),
                    )
                  : _list.isEmpty
                      ? const Center(child: Text('暂无 CDN 云服务商'))
                      : RefreshIndicator(
                          onRefresh: _load,
                          child: ListView.builder(
                            physics: const AlwaysScrollableScrollPhysics(),
                            itemCount: _list.length,
                            itemBuilder: (context, i) {
                              final p = _list[i];
                              return ListTile(
                                title: Text(p.name),
                                subtitle: Column(
                                  crossAxisAlignment:
                                      CrossAxisAlignment.start,
                                  children: [
                                    Text(p.providerCode),
                                    Text(_summary(p)),
                                    Row(
                                      children: [
                                        TextButton.icon(
                                          icon: const Icon(
                                              Icons.settings_outlined,
                                              size: 18),
                                          label: const Text('配置'),
                                          onPressed: () =>
                                              _showConfigSheet(p),
                                        ),
                                        TextButton.icon(
                                          icon: const Icon(
                                              Icons.terminal_outlined,
                                              size: 18),
                                          label: const Text('生成命令'),
                                          onPressed: () => _showPlan(p),
                                        ),
                                      ],
                                    ),
                                  ],
                                ),
                                trailing: Switch(
                                  value: p.enabled,
                                  onChanged: (v) => _toggle(p, v),
                                ),
                              );
                            },
                          ),
                        ),
        ),
      ],
    );
  }
}
