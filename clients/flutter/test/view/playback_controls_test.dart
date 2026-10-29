import 'package:flutter_test/flutter_test.dart';
import 'package:shadowmask/model/session/selected_tracks.dart';
import 'package:shadowmask/model/session/subtitle_selection.dart';
import 'package:shadowmask/view/playback_controls.dart';
import 'package:shadowmask/view/sound_settings.dart';
import 'package:shadowmask/view/subtitle_choice.dart';

void main() {
  group('PlaybackControls', () {
    test('builds a full /update body', () {
      final Map<String, dynamic> body = const PlaybackControls(
        height: 720,
        burn: true,
        downmix: true,
        audioTrack: 1,
        subtitle: SubtitleChoice.on(SubtitleSelection.embedded(2)),
        offsetMs: -250,
      ).toUpdateBody();
      expect(body['target_height'], 720);
      expect(body['force_burn'], true);
      expect(body['downmix_stereo'], true);
      expect(body['audio_track'], 1);
      expect(body['subtitle'], <String, dynamic>{
        'action': 'set',
        'track': <String, dynamic>{'type': 'embedded', 'index': 2},
        'offset_ms': -250,
      });
    });

    test('the delivery preference is stated on an update when set, auto '
        'included', () {
      expect(
        const PlaybackControls().toUpdateBody().containsKey('delivery'),
        isFalse,
      );
      expect(
        const PlaybackControls(
          delivery: DeliveryPreference.auto,
        ).toUpdateBody()['delivery'],
        'auto',
      );
      expect(
        const PlaybackControls(
          delivery: DeliveryPreference.always,
        ).toUpdateBody()['delivery'],
        'always',
      );
    });

    test('the delivery preference is sent on start only when chosen', () {
      expect(
        const PlaybackControls().toStartBody().containsKey('delivery'),
        isFalse,
      );
      expect(
        const PlaybackControls(
          delivery: DeliveryPreference.never,
        ).toStartBody()['delivery'],
        'never',
      );
    });

    test('the delivery preference round trips through the query', () {
      expect(
        PlaybackControls.fromQuery(<String, String>{
          'delivery': 'never',
        }).delivery,
        DeliveryPreference.never,
      );
      expect(
        PlaybackControls.fromQuery(<String, String>{
          'delivery': 'nonsense',
        }).delivery,
        DeliveryPreference.auto,
      );
      expect(
        const PlaybackControls(
          delivery: DeliveryPreference.always,
        ).toQuery()['delivery'],
        'always',
      );
      expect(const PlaybackControls().toQuery().containsKey('delivery'), false);
    });

    test('a link without a converting choice leaves it to the default', () {
      expect(PlaybackControls.fromQuery(<String, String>{}).delivery, isNull);
      expect(
        PlaybackControls.fromQuery(<String, String>{
          'delivery': 'auto',
        }).delivery,
        DeliveryPreference.auto,
      );
      expect(
        const PlaybackControls(
          delivery: DeliveryPreference.auto,
        ).toQuery()['delivery'],
        'auto',
        reason: 'Auto chosen over a saved Always has to survive a reload',
      );
    });

    test('the saved default fills only an unchosen converting setting', () {
      expect(
        const PlaybackControls(
          height: 720,
        ).withDefaultDelivery(DeliveryPreference.always),
        const PlaybackControls(
          height: 720,
          delivery: DeliveryPreference.always,
        ),
      );
      expect(
        const PlaybackControls(
          delivery: DeliveryPreference.never,
        ).withDefaultDelivery(DeliveryPreference.always).delivery,
        DeliveryPreference.never,
      );
    });

    test('disables subtitles and omits audio when unset', () {
      final Map<String, dynamic> body = const PlaybackControls().toUpdateBody();
      expect(body['target_height'], isNull);
      expect(body['subtitle'], <String, dynamic>{'action': 'disable'});
      expect(body.containsKey('audio_track'), isFalse);
    });

    test('query round trips', () {
      final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{
        'height': '480',
        'burn': '1',
        'audio': '3',
        'sub': 'file:z',
        'offset': '100',
      });
      expect(c.height, 480);
      expect(c.burn, isTrue);
      expect(c.downmix, isNull);
      expect(c.audioTrack, 3);
      expect(c.subtitle.selection?.toWire(), 'file:z');
      expect(c.offsetMs, 100);
      expect(c.toQuery(), <String, String?>{
        'height': '480',
        'burn': '1',
        'audio': '3',
        'sub': 'file:z',
        'offset': '100',
      });
    });

    test('a carried language round trips through the query', () {
      final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{
        'alang': 'fr',
        'slang': 'en',
      });
      expect(c.audioLanguage, 'fr');
      expect(c.subtitleLanguage, 'en');
      expect(c.audioTrack, isNull);
      expect(c.subtitle, const SubtitleChoice.auto());
      expect(c.toQuery(), <String, String?>{'alang': 'fr', 'slang': 'en'});
    });

    test('subtitles switched off round trip as a choice, not an absence', () {
      final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{
        'soff': '1',
      });
      expect(c.subtitle, const SubtitleChoice.off());
      expect(c.hasTrackRequest, isTrue);
      expect(c.toStartBody()['subtitle_off'], true);
      expect(c.toQuery(), <String, String?>{'soff': '1'});
    });

    test(
      'an old address with both a subtitle and off reads as the subtitle',
      () {
        final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{
          'sub': 'embedded:2',
          'soff': '1',
        });

        expect(
          c.subtitle,
          const SubtitleChoice.on(SubtitleSelection.embedded(2)),
        );
        expect(c.toStartBody().containsKey('subtitle_off'), isFalse);
        expect(c.toQuery(), <String, String?>{'sub': 'embedded:2'});
      },
    );

    test('any one track choice is a track request', () {
      for (final PlaybackControls one in const <PlaybackControls>[
        PlaybackControls(audioTrack: 1),
        PlaybackControls(audioLanguage: 'fr'),
        PlaybackControls(
          subtitle: SubtitleChoice.on(SubtitleSelection.embedded(2)),
        ),
        PlaybackControls(subtitleLanguage: 'en'),
        PlaybackControls(subtitle: SubtitleChoice.off()),
      ]) {
        expect(one.hasTrackRequest, isTrue, reason: '$one');
      }
    });

    test('the default carries no query', () {
      expect(const PlaybackControls().toQuery(), isEmpty);
      expect(const PlaybackControls().hasTrackRequest, isFalse);
      expect(const PlaybackControls().toStartBody(), isEmpty);
    });

    test('builds a /sessions body from an index or a language, not both', () {
      expect(
        const PlaybackControls(
          height: 720,
          burn: true,
          downmix: true,
          audioTrack: 1,
          subtitle: SubtitleChoice.on(SubtitleSelection.file('s9')),
        ).toStartBody(),
        <String, dynamic>{
          'target_height': 720,
          'force_burn': true,
          'downmix_stereo': true,
          'audio_track': 1,
          'subtitle': <String, dynamic>{
            'track': <String, dynamic>{'type': 'file', 'id': 's9'},
          },
        },
      );
      expect(
        const PlaybackControls(
          audioLanguage: 'fr',
          subtitleLanguage: 'fr',
        ).toStartBody(),
        <String, dynamic>{'audio_language': 'fr', 'subtitle_language': 'fr'},
      );
    });

    // POST /sessions reads subtitle as {track, offset_ms}, the same shape the
    // update body sends. A bare track ref there is a 422 the player only meets
    // on a reload, because the subtitle rides in the URL.
    test('wraps the subtitle as a selection and carries its offset', () {
      expect(
        const PlaybackControls(
          subtitle: SubtitleChoice.on(SubtitleSelection.embedded(2)),
          offsetMs: 250,
        ).toStartBody()['subtitle'],
        <String, dynamic>{
          'track': <String, dynamic>{'type': 'embedded', 'index': 2},
          'offset_ms': 250,
        },
      );
    });

    test('the negotiated selection fills the indices and keeps the carry', () {
      const PlaybackControls asked = PlaybackControls(
        height: 480,
        audioLanguage: 'fr',
        subtitleLanguage: 'fr',
        offsetMs: 50,
      );
      final PlaybackControls got = asked.withSelection(
        const SelectedTracks(
          audioTrack: 4,
          subtitleTrack: SubtitleSelection.embedded(6),
        ),
      );
      expect(got.audioTrack, 4);
      expect(
        got.subtitle,
        const SubtitleChoice.on(SubtitleSelection.embedded(6)),
      );
      expect(got.height, 480);
      expect(got.offsetMs, 50);
      expect(
        got.audioLanguage,
        'fr',
        reason:
            'the language is what travels to the next episode, so resolving it '
            'here must not consume it',
      );
      expect(got.subtitleLanguage, 'fr');
    });

    test('a request the server could not honour leaves the carry standing', () {
      const PlaybackControls asked = PlaybackControls(subtitleLanguage: 'fr');
      final PlaybackControls got = asked.withSelection(const SelectedTracks());
      expect(got.subtitle, const SubtitleChoice.auto());
      expect(
        got.subtitleLanguage,
        'fr',
        reason:
            'this episode has no French, but the one after it might, and the '
            'viewer never withdrew the request',
      );
    });

    test('equality covers every field', () {
      const PlaybackControls base = PlaybackControls();
      expect(base, const PlaybackControls());
      expect(base.hashCode, const PlaybackControls().hashCode);
      for (final PlaybackControls other in <PlaybackControls>[
        const PlaybackControls(height: 720),
        const PlaybackControls(burn: true),
        const PlaybackControls(downmix: true),
        const PlaybackControls(audioTrack: 1),
        const PlaybackControls(audioLanguage: 'en'),
        const PlaybackControls(
          subtitle: SubtitleChoice.on(SubtitleSelection.embedded(1)),
        ),
        const PlaybackControls(subtitleLanguage: 'en'),
        const PlaybackControls(subtitle: SubtitleChoice.off()),
        const PlaybackControls(offsetMs: 5),
        const PlaybackControls(delivery: DeliveryPreference.auto),
        const PlaybackControls(normalize: true),
        const PlaybackControls(dialogue: DialogueLift.none),
        const PlaybackControls(night: NightMode.off),
      ]) {
        expect(base, isNot(other));
      }
    });

    test('copies keep every field they were not asked to change', () {
      const PlaybackControls full = PlaybackControls(
        height: 720,
        burn: true,
        downmix: true,
        audioTrack: 1,
        audioLanguage: 'en',
        subtitle: SubtitleChoice.on(SubtitleSelection.embedded(2)),
        subtitleLanguage: 'fr',
        offsetMs: 5,
        delivery: DeliveryPreference.never,
        normalize: false,
        dialogue: DialogueLift.db6,
        night: NightMode.light,
      );

      expect(full.withSelection(null), full);
      expect(
        full.withDefaultSound(
          downmix: false,
          normalize: true,
          dialogue: DialogueLift.none,
          night: NightMode.strong,
        ),
        full,
      );
      expect(
        full.withSelection(
          const SelectedTracks(
            audioTrack: 3,
            subtitleTrack: SubtitleSelection.embedded(4),
          ),
        ),
        full.copyWith(
          audioTrack: 3,
          subtitle: const SubtitleChoice.on(SubtitleSelection.embedded(4)),
        ),
      );
      expect(full.withDefaultDelivery(DeliveryPreference.always), full);
      expect(
        full
            .copyWith(delivery: null)
            .withDefaultDelivery(DeliveryPreference.always),
        full.copyWith(delivery: DeliveryPreference.always),
      );
    });

    test('two controls are the same update when their bodies match', () {
      const PlaybackControls none = PlaybackControls();

      expect(
        const PlaybackControls(delivery: DeliveryPreference.auto).sameUpdateAs(
          const PlaybackControls(delivery: DeliveryPreference.never),
        ),
        isFalse,
      );
      expect(
        none.sameUpdateAs(
          const PlaybackControls(audioLanguage: 'en', subtitleLanguage: 'fr'),
        ),
        isTrue,
      );
      expect(
        none.sameUpdateAs(const PlaybackControls(offsetMs: 1500)),
        isTrue,
        reason: 'an offset with no subtitle is not sent',
      );
      expect(none.sameUpdateAs(const PlaybackControls(audioTrack: 1)), isFalse);
      expect(
        const PlaybackControls(
          subtitle: SubtitleChoice.on(SubtitleSelection.embedded(2)),
        ).sameUpdateAs(
          const PlaybackControls(
            subtitle: SubtitleChoice.on(SubtitleSelection.embedded(2)),
            offsetMs: 1500,
          ),
        ),
        isFalse,
      );
    });
  });

  group('sound settings', () {
    test('an update states every one, the server defaults when unset', () {
      expect(
        const PlaybackControls(
          normalize: false,
          dialogue: DialogueLift.db6,
          night: NightMode.strong,
        ).toUpdateBody(),
        containsPair('normalize_downmix', false),
      );
      final Map<String, dynamic> stated = const PlaybackControls(
        downmix: true,
        normalize: false,
        dialogue: DialogueLift.db6,
        night: NightMode.strong,
      ).toUpdateBody();
      expect(stated['downmix_stereo'], true);
      expect(stated['dialogue_db'], 6);
      expect(stated['night'], 'strong');
      final Map<String, dynamic> unset = const PlaybackControls()
          .toUpdateBody();
      expect(unset['downmix_stereo'], false);
      expect(unset['normalize_downmix'], true);
      expect(unset['dialogue_db'], 0);
      expect(unset['night'], 'off');
    });

    test('a start states only what differs from the server default', () {
      expect(
        const PlaybackControls(
          downmix: false,
          normalize: true,
          dialogue: DialogueLift.none,
          night: NightMode.off,
        ).toStartBody(),
        isEmpty,
      );
      expect(
        const PlaybackControls(
          downmix: true,
          normalize: false,
          dialogue: DialogueLift.db3,
          night: NightMode.light,
        ).toStartBody(),
        <String, dynamic>{
          'downmix_stereo': true,
          'normalize_downmix': false,
          'dialogue_db': 3,
          'night': 'light',
        },
      );
    });

    test('they round trip through the query, a switch turned off included', () {
      final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{
        'downmix': '0',
        'normalize': '0',
        'dialogue': '9',
        'night': 'strong',
      });
      expect(c.downmix, isFalse);
      expect(c.normalize, isFalse);
      expect(c.dialogue, DialogueLift.db9);
      expect(c.night, NightMode.strong);
      expect(c.toQuery(), <String, String?>{
        'downmix': '0',
        'normalize': '0',
        'dialogue': '9',
        'night': 'strong',
      });
    });

    test('a link without them leaves them to the device', () {
      final PlaybackControls c = PlaybackControls.fromQuery(<String, String>{});
      expect(c.downmix, isNull);
      expect(c.normalize, isNull);
      expect(c.dialogue, isNull);
      expect(c.night, isNull);
      final PlaybackControls odd = PlaybackControls.fromQuery(<String, String>{
        'dialogue': '5',
        'night': 'loud',
      });
      expect(odd.dialogue, DialogueLift.none);
      expect(odd.night, NightMode.off);
    });

    test('the device defaults fill only what the session has not chosen', () {
      expect(
        const PlaybackControls(night: NightMode.off).withDefaultSound(
          downmix: true,
          normalize: false,
          dialogue: DialogueLift.db6,
          night: NightMode.strong,
        ),
        const PlaybackControls(
          downmix: true,
          normalize: false,
          dialogue: DialogueLift.db6,
          night: NightMode.off,
        ),
      );
    });

    test('normalize and the lift are the tuning; the rest is not', () {
      const PlaybackControls plain = PlaybackControls(
        normalize: true,
        dialogue: DialogueLift.none,
        night: NightMode.off,
      );
      final PlaybackControls tuned = plain.copyWith(
        normalize: false,
        dialogue: DialogueLift.db6,
      );
      expect(tuned.sameTuningAs(plain), isFalse);
      expect(tuned.withoutTuning.sameUpdateAs(plain.withoutTuning), isTrue);
      expect(const PlaybackControls().sameTuningAs(plain), isTrue);
      final PlaybackControls night = plain.copyWith(night: NightMode.light);
      expect(night.sameTuningAs(plain), isTrue);
      expect(night.withoutTuning.sameUpdateAs(plain.withoutTuning), isFalse);
    });
  });
}
