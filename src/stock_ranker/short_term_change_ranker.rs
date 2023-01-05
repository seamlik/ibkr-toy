use super::FactorRanker;
use super::Name;
use super::Score;
use super::ScoringFactor;
use super::StockCandidates;
use std::collections::HashMap;

#[mockall_double::double]
use super::notional_ranker::NotionalRanker;

#[derive(Default)]
pub struct ShortTermChangeRanker {
    notional_ranker: NotionalRanker,
}

impl FactorRanker for ShortTermChangeRanker {
    fn rank(&self, candidates: &StockCandidates) -> HashMap<Name, Score> {
        let notional_candidates: HashMap<_, _> = candidates
            .iter()
            .filter_map(|(name, factors)| {
                factors
                    .get(&ScoringFactor::ShortTermChange)
                    .filter(|notional| notional.value < 0.0)
                    .map(|notional| notional.value.abs().into())
                    .map(|notional| (name.clone(), notional))
            })
            .collect();
        self.notional_ranker.rank(&notional_candidates)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::stock_ranker::Notional;

    #[test]
    fn rank_correct_candidates() {
        // Given
        let stock_candidates: StockCandidates = [
            (
                "A",
                HashMap::from([(ScoringFactor::ShortTermChange, Notional::from(-1.0))]),
            ),
            (
                "B",
                HashMap::from([(ScoringFactor::LongTermChange, Notional::from(-1.0))]),
            ),
            (
                "C",
                HashMap::from([(ScoringFactor::ShortTermChange, Notional::from(1.0))]),
            ),
            (
                "D",
                HashMap::from([(ScoringFactor::ShortTermChange, Notional::from(0.0))]),
            ),
        ]
        .into();
        let expected_notional_candidates: HashMap<_, _> = [("A".into(), 1.0.into())].into();
        let dummy_scores = HashMap::default();
        let mut notional_ranker = NotionalRanker::default();
        notional_ranker
            .expect_rank()
            .withf_st(move |arg| arg == &expected_notional_candidates)
            .return_const_st(dummy_scores.clone());
        let service = ShortTermChangeRanker { notional_ranker };

        // When
        let actual_scores = service.rank(&stock_candidates);

        // Then
        assert_eq!(dummy_scores, actual_scores);
    }

    #[test]
    fn rank_no_candidate() {
        // Given
        let stock_candidates = StockCandidates::default();
        let expected_notional_candidates = HashMap::default();
        let dummy_scores = HashMap::default();
        let mut notional_ranker = NotionalRanker::default();
        notional_ranker
            .expect_rank()
            .withf_st(move |arg| arg == &expected_notional_candidates)
            .return_const_st(dummy_scores.clone());
        let service = ShortTermChangeRanker { notional_ranker };

        // When
        let actual_scores = service.rank(&stock_candidates);

        // Then
        assert_eq!(dummy_scores, actual_scores);
    }
}
