import 'package:flutter/material.dart';

import '../models/travel_models.dart';
import '../services/content_service.dart';
import '../services/localization_service.dart';
import '../services/order_service.dart';
import 'destination_detail_page.dart';

/// 搜索页：关键词 + 目的地筛选 + 价格区间，结果混合目的地/景点。
class SearchPage extends StatefulWidget {
  const SearchPage({super.key});

  @override
  State<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<SearchPage> {
  final _keyword = TextEditingController();
  final _priceMin = TextEditingController();
  final _priceMax = TextEditingController();
  List<Destination> _destinations = [];
  Destination? _selectedDestination;
  List<SearchItem> _results = [];
  bool _loading = false;
  bool _searched = false;
  String? _error;
  String _loadedLang = '';

  @override
  void initState() {
    super.initState();
    _loadedLang = LocalizationService.instance.locale.languageCode;
    _fetchDestinations();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final lang = LocalizationService.instance.locale.languageCode;
    if (lang != _loadedLang) {
      _loadedLang = lang;
      if (_searched) _search();
    }
  }

  @override
  void dispose() {
    _keyword.dispose();
    _priceMin.dispose();
    _priceMax.dispose();
    super.dispose();
  }

  Future<void> _fetchDestinations() async {
    try {
      final list = await ContentService.instance.fetchDestinations();
      if (mounted && list.isNotEmpty) setState(() => _destinations = list);
    } on Exception {
      // ponytail: 筛选下拉拿不到目的地时隐藏，搜索仍可用关键词
    }
  }

  Future<void> _search() async {
    final q = _keyword.text.trim();
    if (q.isEmpty && _selectedDestination == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(LocalizationService.instance.getString('search.hint'))),
      );
      return;
    }
    FocusScope.of(context).unfocus();
    setState(() {
      _loading = true;
      _error = null;
      _searched = true;
    });
    try {
      final result = await OrderService.instance.search(
        q: q,
        destinationId: _selectedDestination?.id,
        priceMin: int.tryParse(_priceMin.text),
        priceMax: int.tryParse(_priceMax.text),
      );
      if (!mounted) return;
      setState(() {
        _results = result.items;
        _loading = false;
      });
    } on Exception {
      if (!mounted) return;
      setState(() {
        _error = LocalizationService.instance.getString('common.loadFailed');
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(loc.getString('nav.search'), style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _keyword,
                textInputAction: TextInputAction.search,
                onSubmitted: (_) => _search(),
                decoration: InputDecoration(
                  hintText: loc.getString('search.placeholder'),
                  prefixIcon: const Icon(Icons.search),
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                ),
              ),
            ),
            const SizedBox(width: 8),
            FilledButton(onPressed: _search, child: Text(loc.getString('search.button'))),
          ],
        ),
        const SizedBox(height: 12),
        if (_destinations.isNotEmpty) ...[
          DropdownButtonFormField<Destination>(
            initialValue: _selectedDestination,
            decoration: InputDecoration(
              labelText: loc.getString('search.allDestinations'),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
            ),
            items: [
              for (final d in _destinations) DropdownMenuItem(value: d, child: Text(d.name)),
            ],
            onChanged: (d) => setState(() => _selectedDestination = d),
          ),
          const SizedBox(height: 12),
        ],
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _priceMin,
                keyboardType: TextInputType.number,
                decoration: InputDecoration(
                  labelText: loc.getString('search.minPrice'),
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                ),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: TextField(
                controller: _priceMax,
                keyboardType: TextInputType.number,
                decoration: InputDecoration(
                  labelText: loc.getString('search.maxPrice'),
                  border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        if (_loading)
          const Padding(
            padding: EdgeInsets.all(24),
            child: Center(child: CircularProgressIndicator()),
          )
        else if (_error != null)
          Column(
            children: [
              Text(_error!),
              TextButton(onPressed: _search, child: Text(loc.getString('common.retry'))),
            ],
          )
        else if (_searched && _results.isEmpty)
          Padding(
            padding: const EdgeInsets.all(24),
            child: Center(child: Text(loc.getString('search.empty'))),
          )
        else if (_searched)
          for (final item in _results) _SearchResultCard(item: item),
      ],
    );
  }
}

class _SearchResultCard extends StatelessWidget {
  const _SearchResultCard({required this.item});

  final SearchItem item;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    final icon = item.isDestination ? Icons.location_city : Icons.attractions;
    final typeLabel = loc.getString(
      item.isDestination ? 'search.resultDestination' : 'search.resultAttraction',
    );
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: ListTile(
        leading: item.coverUrl.isNotEmpty
            ? ClipRRect(
                borderRadius: BorderRadius.circular(8),
                child: Image.network(
                  item.coverUrl,
                  width: 56,
                  height: 56,
                  fit: BoxFit.cover,
                  errorBuilder: (_, _, _) => Icon(icon, size: 32),
                ),
              )
            : Icon(icon, size: 40),
        title: Text(item.name),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (item.description.isNotEmpty)
              Text(item.description, maxLines: 2, overflow: TextOverflow.ellipsis),
            Row(
              children: [
                Text(typeLabel, style: Theme.of(context).textTheme.labelSmall),
                const Spacer(),
                if (item.priceCents > 0)
                  Text(
                    formatYuan(item.priceCents),
                    style: Theme.of(context).textTheme.titleSmall?.copyWith(
                          color: Theme.of(context).colorScheme.primary,
                        ),
                  ),
              ],
            ),
          ],
        ),
        onTap: () => Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => item.isDestination
                ? DestinationDetailPage(
                    destination: Destination(
                      id: item.id,
                      name: item.name,
                      coverUrl: item.coverUrl,
                      description: item.description,
                    ),
                  )
                : AttractionDetailPage(
                    attraction: Attraction(
                      id: item.id,
                      destinationId: 0,
                      name: item.name,
                      description: item.description,
                      priceCents: item.priceCents,
                      coverUrl: item.coverUrl,
                    ),
                  ),
          ),
        ),
      ),
    );
  }
}

/// 景区详情：搜索结果的 attraction 类型跳转目标。
class AttractionDetailPage extends StatelessWidget {
  const AttractionDetailPage({super.key, required this.attraction});

  final Attraction attraction;

  @override
  Widget build(BuildContext context) {
    final a = attraction;
    final scheme = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(title: Text(a.name)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child: AspectRatio(
              aspectRatio: 16 / 9,
              child: a.coverUrl.isNotEmpty
                  ? Image.network(a.coverUrl, fit: BoxFit.cover, errorBuilder: (_, _, _) => _placeholder(context))
                  : _placeholder(context),
            ),
          ),
          const SizedBox(height: 12),
          if (a.rating > 0) StarRating(rating: a.rating),
          if (a.description.isNotEmpty) ...[
            const SizedBox(height: 12),
            Text(a.description),
          ],
          const SizedBox(height: 12),
          Wrap(
            spacing: 16,
            runSpacing: 4,
            children: [
              if (a.priceCents > 0)
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.payments, size: 16, color: scheme.primary),
                    const SizedBox(width: 4),
                    Text(formatYuan(a.priceCents)),
                  ],
                ),
              if (a.openHours.isNotEmpty)
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.schedule, size: 16, color: scheme.primary),
                    const SizedBox(width: 4),
                    Text(a.openHours),
                  ],
                ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _placeholder(BuildContext context) => Container(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        child: const Icon(Icons.attractions, size: 48),
      );
}
