import importlib.util
from pathlib import Path
import unittest
import copy
import tempfile
import hashlib

spec=importlib.util.spec_from_file_location('review',Path(__file__).parents[1]/'review-retrieval.py')
review=importlib.util.module_from_spec(spec);spec.loader.exec_module(review)

class ReviewTests(unittest.TestCase):
    def packet(self):
        return {'schema_version':1,'status':'reviewed','query_author':'author-a','reviewer':'reviewer-b',
                'reviewer_attests_independence':True,'reviewed_at':'2026-10-18','review_notes':'test fixture only',
                'queries':[{'id':str(i),'query':'fixture','language':['th','en','mixed','technical'][i%4],
                            'category':['hardware','exact_api','documentation'][i%3],
                            'judgments':{'a.c':2,'b.h':0},'review_notes':'test fixture'} for i in range(15)]}
    def test_drafts_and_assistant_review_are_not_independent_evidence(self):
        for changes in [{'status':'draft'},{'reviewer':'assistant'},{'reviewer':'author-a'},{'reviewer_attests_independence':False},{'reviewer_attests_independence':'false'}]:
            packet=self.packet();packet.update(changes)
            with self.assertRaises(ValueError):review.validate_review(packet,['a.c','b.h'])
    def test_unjudged_files_are_not_silently_treated_as_irrelevant(self):
        packet=self.packet();packet['queries'][0]['judgments'].pop('b.h')
        with self.assertRaises(ValueError):review.validate_review(packet,['a.c','b.h'])
    def test_graded_judgments_export_binary_qrels_without_mutating_input(self):
        packet=self.packet();result=review.validate_review(packet,['a.c','b.h'])
        self.assertEqual(result[0]['relevant_files'],['a.c']);self.assertEqual(packet['queries'][0]['judgments']['a.c'],2)
    def test_category_membership_is_declared_from_query_intent(self):
        self.assertIn('hardware',review.groups({'id':'heldout-pins-th','language':'th'}))
        self.assertIn('exact_api',review.groups({'id':'heldout-symbol-api','language':'technical'}))
    def test_development_gate_requires_full_review_and_excludes_final_data(self):
        packet=self.packet();packet.update(schema_version=2,purpose='retrieval-development')
        packet['queries']=[{**copy.deepcopy(packet['queries'][i%15]),'id':f'dev-{i}','query':f'new question {i}',
                            'query_kind':['exact_symbol','natural_language','mixed_language','compiler_error','hardware'][i%5]} for i in range(20)]
        self.assertEqual(len(review.validate_development_review(packet,['a.c','b.h'],[])),20)
        for altered in [{**packet,'status':'draft'},{**packet,'reviewer':'codex'}]:
            with self.assertRaises(ValueError):review.validate_development_review(altered,['a.c','b.h'],[])
        with self.assertRaises(ValueError):review.validate_development_review(packet,['a.c','b.h'],[{'id':'frozen','query':' NEW QUESTION 0 '}])
        packet['queries'][0]['ranked_files']=['a.c']
        with self.assertRaises(ValueError):review.validate_development_review(packet,['a.c','b.h'],[])
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);source=root/'a.c';source.write_bytes(b'original')
            hashes={'a.c':hashlib.sha256(b'original').hexdigest()}
            review.verify_corpus(root,hashes)
            source.write_bytes(b'changed')
            with self.assertRaises(ValueError):review.verify_corpus(root,hashes)

if __name__=='__main__':unittest.main()
