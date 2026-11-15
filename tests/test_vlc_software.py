import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'scripts'))
from run_event_benchmarks import command_for

class VlcSoftwareTests(unittest.TestCase):
    def test_software_field_disallows_platform_decoder_fallback(self):
        args = command_for('vlc', 'vlc', 'hevc4k', Path('fixture.mkv'), None, 1234, 'test')
        self.assertIn('--codec=avcodec,none', args)
        self.assertIn('--avcodec-hw=none', args)
        self.assertIn('--verbose=2', args)
        native = command_for('vlc', 'vlc', 'hevc4k', Path('fixture.mkv'), None, 1234, 'test', 'native-window')
        self.assertFalse(any(a.startswith('--codec=') for a in native))
        self.assertIn('--avcodec-hw=any', native)

    def test_positive_decoder_evidence_is_required_for_software_medals(self):
        from event_benchmarks import vlc_software_decode_verified
        software = '[abc] main decoder debug: using video decoder module "avcodec"\n'
        self.assertTrue(vlc_software_decode_verified(software, 'hevc4k'))
        self.assertFalse(vlc_software_decode_verified('', 'hevc4k'))
        self.assertFalse(vlc_software_decode_verified('[abc] videotoolbox decoder: Using Video Toolbox to decode', 'hevc4k'))
        self.assertFalse(vlc_software_decode_verified(software + '[abc] main generic debug: using hw decoder module "d3d11va"', 'hevc4k'))
        self.assertFalse(vlc_software_decode_verified(software + '[abc] avcodec decoder: Using D3D11VA for hardware decoding', 'hevc4k'))
        self.assertFalse(vlc_software_decode_verified(software + '[abc] main decoder debug: using video decoder module "videotoolbox"', 'hevc4k'))
        self.assertFalse(vlc_software_decode_verified(software, 'flac'))
        self.assertTrue(vlc_software_decode_verified(software.replace('video decoder', 'audio decoder'), 'flac'))

    def test_archived_software_rows_without_decoder_proof_cannot_keep_medals(self):
        from refresh_scoreboards import enforce_software_decoder_evidence
        import copy
        base = dict(session=dict(presentation='software-null'), cases=[dict(player='vlc', profile='old', trial=0, verified=True, status='verified-playback')], measurements=[dict(player='vlc', profile='old', trial=0, verified=True), dict(player='mpv', profile='old', trial=0, verified=True)])
        old = copy.deepcopy(base)
        enforce_software_decoder_evidence(old)
        self.assertFalse(old['cases'][0]['verified'])
        self.assertFalse(old['measurements'][0]['verified'])
        self.assertTrue(old['measurements'][1]['verified'])
        proven = copy.deepcopy(base)
        proven['cases'][0]['software_decode_verified'] = True
        enforce_software_decoder_evidence(proven)
        self.assertTrue(proven['measurements'][0]['verified'])
        native = copy.deepcopy(base)
        native['session']['presentation'] = 'native-window'
        enforce_software_decoder_evidence(native)
        self.assertTrue(native['measurements'][0]['verified'])
        missing = copy.deepcopy(base)
        missing['cases'] = []
        enforce_software_decoder_evidence(missing)
        self.assertFalse(missing['measurements'][0]['verified'])
