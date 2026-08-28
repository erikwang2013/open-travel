import 'package:flutter/material.dart';

import '../services/localization_service.dart';
import 'home_page.dart';
import 'orders_page.dart';
import 'profile_page.dart';
import 'search_page.dart';

class HomeShell extends StatefulWidget {
  const HomeShell({super.key});

  @override
  State<HomeShell> createState() => _HomeShellState();
}

class _HomeShellState extends State<HomeShell> {
  int _index = 0;

  @override
  Widget build(BuildContext context) {
    final loc = LocalizationService.instance;
    const pages = [HomePage(), SearchPage(), OrdersPage(), ProfilePage()];
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth > 800) {
          return Scaffold(
            body: Row(
              children: [
                NavigationRail(
                  selectedIndex: _index,
                  onDestinationSelected: (i) => setState(() => _index = i),
                  labelType: NavigationRailLabelType.all,
                  destinations: [
                    NavigationRailDestination(
                      icon: const Icon(Icons.home_outlined),
                      selectedIcon: const Icon(Icons.home),
                      label: Text(loc.getString('nav.home')),
                    ),
                    NavigationRailDestination(
                      icon: const Icon(Icons.search),
                      label: Text(loc.getString('nav.search')),
                    ),
                    NavigationRailDestination(
                      icon: const Icon(Icons.card_travel_outlined),
                      selectedIcon: const Icon(Icons.card_travel),
                      label: Text(loc.getString('nav.bookings')),
                    ),
                    NavigationRailDestination(
                      icon: const Icon(Icons.person_outline),
                      selectedIcon: const Icon(Icons.person),
                      label: Text(loc.getString('nav.profile')),
                    ),
                  ],
                ),
                const VerticalDivider(width: 1),
                Expanded(child: IndexedStack(index: _index, children: pages)),
              ],
            ),
          );
        }
        return Scaffold(
          body: IndexedStack(index: _index, children: pages),
          bottomNavigationBar: NavigationBar(
            selectedIndex: _index,
            onDestinationSelected: (i) => setState(() => _index = i),
            destinations: [
              NavigationDestination(
                icon: const Icon(Icons.home_outlined),
                selectedIcon: const Icon(Icons.home),
                label: loc.getString('nav.home'),
              ),
              NavigationDestination(
                icon: const Icon(Icons.search),
                label: loc.getString('nav.search'),
              ),
              NavigationDestination(
                icon: const Icon(Icons.card_travel_outlined),
                selectedIcon: const Icon(Icons.card_travel),
                label: loc.getString('nav.bookings'),
              ),
              NavigationDestination(
                icon: const Icon(Icons.person_outline),
                selectedIcon: const Icon(Icons.person),
                label: loc.getString('nav.profile'),
              ),
            ],
          ),
        );
      },
    );
  }
}
