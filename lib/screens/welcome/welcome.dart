import 'package:fluent_ui/fluent_ui.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:player/utils/ax_shadow.dart';

class WelcomePage extends StatefulWidget {
  const WelcomePage({super.key});

  @override
  State<WelcomePage> createState() => _WelcomePageState();
}

class _WelcomePageState extends State<WelcomePage> {
  @override
  Widget build(BuildContext context) {
    final theme = FluentTheme.of(context);

    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 400, maxHeight: 560),
        child: Container(
            decoration: BoxDecoration(
              color: theme.cardColor,
              borderRadius: BorderRadius.circular(3),
              boxShadow: axShadow(20),
            ),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child:
                  Column(mainAxisAlignment: MainAxisAlignment.start, children: [
                Expanded(
                    child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    SvgPicture.asset(
                      'assets/mono_color_logo.svg',
                      colorFilter:
                          ColorFilter.mode(theme.activeColor, BlendMode.srcIn),
                    ),
                    const SizedBox(
                      height: 20,
                    ),
                    Column(
                      children: [
                        const Padding(
                          padding: EdgeInsets.symmetric(horizontal: 24),
                          child: Text(
                            'Select your audio library directory, and we will scan and analysis all tracks within it.',
                            textAlign: TextAlign.center,
                            style: TextStyle(
                              height: 1.4,
                            ),
                          ),
                        ),
                        const SizedBox(
                          height: 56,
                        ),
                        FilledButton(
                            child: const Text("Select Directory"),
                            onPressed: () => {})
                      ],
                    )
                  ],
                )),
                Text(
                  '© 2024 Rune Player Developers. Licensed under MPL 2.0.',
                  style: theme.typography.caption
                      ?.apply(color: theme.activeColor.withAlpha(80)),
                ),
              ]),
            )),
      ),
    );
  }
}
