import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('diagnosis', Path(__file__).parents[1]/'diagnose-retrieval.py')
MODULE = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(MODULE)

class DiagnosisTests(unittest.TestCase):
    def test_duplicate_chunk_metrics_match_the_frozen_contract(self):
        value = MODULE.metrics(['b', 'b', 'a'], ['a', 'b'])
        self.assertEqual(value['recall_at_5'], 1)
        self.assertEqual(value['precision_at_5'], .4)
        self.assertEqual(value['mrr'], 1)
        self.assertAlmostEqual(value['ndcg_at_5'], 1)

    def test_diagnosis_rejects_changed_labels_and_reports_regressions_without_mutation(self):
        case = {'id':'sample','query':'Thai API usage','language':'en','relevant_files':['a']}
        row = {**case,'ranked_files':['a','b'],'metrics':MODULE.metrics(['a','b'],['a']), 'latency_ms':10,
               'query_plan':{'original':case['query'],'concepts':['api']}}
        before = {'queries':[row]};after = copy.deepcopy(before)
        after['queries'][0].update(ranked_files=['b','b','a'],metrics=MODULE.metrics(['b','b','a'],['a']))
        saved = copy.deepcopy(after)
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory)/'a').write_text('reference')
            result = MODULE.diagnose([case],before,after,Path(directory))
            self.assertEqual(result['flags']['reciprocal_rank_regression'], 1)
            self.assertEqual(result['flags']['duplicate_file_slots'], 1)
            self.assertIsNone(result['query_kinds']['compiler_error']['metrics'])
            self.assertEqual(saved,after)
            changed = copy.deepcopy(after);changed['queries'][0]['relevant_files'] = ['b']
            with self.assertRaises(ValueError): MODULE.diagnose([case],before,changed,Path(directory))

    def test_categories_are_analysis_only_and_overlap_explicitly(self):
        self.assertEqual(MODULE.kinds({'id':'x','language':'technical','query':'kb_ascii_parse'}),['exact_symbol'])
        self.assertIn('compiler_error',MODULE.kinds({'id':'x','language':'en','query':"undefined reference to 'api'"}))
        self.assertEqual(set(MODULE.kinds({'id':'heldout-pins-mixed','language':'mixed','query':'ขา GPIO'})),{'mixed_language','hardware'})
