import 'package:flutter/material.dart';

import '../api.dart';
import 'attractions_page.dart';
import 'destinations_page.dart';
import 'lines_page.dart';

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  int _index = 0;

  static const _pages = [DestinationsPage(), AttractionsPage(), LinesPage()];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Open Travel 管理端')),
      body: Row(
        children: [
          NavigationRail(
            selectedIndex: _index,
            onDestinationSelected: (i) => setState(() => _index = i),
            destinations: const [
              NavigationRailDestination(
                  icon: Icon(Icons.place_outlined),
                  selectedIcon: Icon(Icons.place),
                  label: Text('目的地管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.attractions_outlined),
                  selectedIcon: Icon(Icons.attractions),
                  label: Text('景区管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.route_outlined),
                  selectedIcon: Icon(Icons.route),
                  label: Text('线路管理')),
            ],
            trailing: Padding(
              padding: const EdgeInsets.only(top: 12),
              child: IconButton(
                tooltip: '退出登录',
                icon: const Icon(Icons.logout),
                onPressed: () => AuthService.instance.logout(),
              ),
            ),
          ),
          const VerticalDivider(width: 1),
          Expanded(
            child: IndexedStack(index: _index, children: _pages),
          ),
        ],
      ),
    );
  }
}
