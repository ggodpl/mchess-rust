use crate::r#const::{CASTLING_VALUE, CHECK_VALUE, DEFAULT_MARGIN, KILLER_MOVE_VALUE, MAX_WINDOW_WIDTH, PAWN_DEVELOPMENT_BONUS, PROMOTION_VALUE, PV_MOVE};
use crate::evaluation::{evaluate, EvaluationResult};
use crate::board::{Board, ResultType};
use crate::moves::{Move, MoveType};
use crate::piece::PieceType;
use core::f64;
use std::collections::HashMap;

pub struct Minimax {
    evaluation_cache: EvalCache,
    move_evaluation_cache: HashMap<usize, f64>,
    transposition_table: TranspositionTable,
    killer_moves: Vec<Vec<Option<Move>>>,
    pub nodes: u64,
    is_stopping: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType {
    PV,
    Cut,
    All
}

#[derive(Debug, Clone)]
pub struct Node {
    depth: u8,
    node_type: NodeType,
    score: f64,
    best_move: Option<Move>
}

#[derive(Debug)]
pub struct SearchResult {
    pub value: f64,
    pub moves: Vec<Move>
}

pub struct TranspositionTable {
    entries: Vec<Option<Node>>,
    mask: usize
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let num_entries = (size_mb * 1024 * 1024) / std::mem::size_of::<Option<Node>>();
        let size = num_entries.next_power_of_two();
        TranspositionTable { 
            entries: vec![None; size], 
            mask: size - 1
        }
    }

    pub fn store(&mut self, hash: i64, node: Node) {
        let index = (hash as usize) & self.mask;
        if let Some(entry) = &self.entries[index] {
            if entry.depth <= node.depth {
                self.entries[index] = Some(node);
            }
        } else {
            self.entries[index] = Some(node);
        }
    }

    pub fn get(&self, hash: i64) -> &Option<Node> {
        let index = (hash as usize) & self.mask;
        &self.entries[index]
    }
}

pub struct EvalCache {
    entries: Vec<(i64, EvaluationResult)>,  // (hash, result)
    mask: usize
}

impl EvalCache {
    pub fn new(size_mb: usize) -> Self {
        let num_entries = (size_mb * 1024 * 1024) / std::mem::size_of::<(i64, EvaluationResult)>();
        let size = num_entries.next_power_of_two();
        EvalCache { 
            entries: vec![(0, EvaluationResult::default()); size], 
            mask: size - 1
        }
    }
    
    pub fn get(&self, hash: i64) -> Option<&EvaluationResult> {
        let index = (hash as usize) & self.mask;
        let (stored_hash, result) = &self.entries[index];
        if *stored_hash == hash {
            Some(result)
        } else {
            None
        }
    }
    
    pub fn store(&mut self, hash: i64, result: EvaluationResult) {
        let index = (hash as usize) & self.mask;
        self.entries[index] = (hash, result);
    }

    pub fn contains(&self, hash: i64) -> bool {
        let index = (hash as usize) & self.mask;
        self.entries[index].0 == hash
    }
}

impl Minimax {
    pub fn new() -> Self {
        Minimax {
            evaluation_cache: EvalCache::new(64),
            move_evaluation_cache: HashMap::new(),
            transposition_table: TranspositionTable::new(64),
            killer_moves: vec![vec![None; 2]; 100],
            nodes: 0,
            is_stopping: false
        }
    }

    pub fn store_position(&mut self, board: &Board, depth: u8, node_type: NodeType, score: f64, best_move: Option<Move>) {
        let node = Node {
            depth,
            node_type,
            score,
            best_move
        };

        self.transposition_table.store(board.hash, node);
    }

    pub fn check_position(&self, board: &Board, depth: u8, alpha: f64, beta: f64) -> Option<(f64, Option<Move>)> {
        if let Some(node) = self.transposition_table.get(board.hash) {
            if node.depth >= depth {
                match node.node_type {
                    NodeType::PV => return Some((node.score, node.best_move.clone())),
                    NodeType::Cut if node.score >= beta => return Some((beta, node.best_move.clone())),
                    NodeType::All if node.score <= alpha => return Some((alpha, node.best_move.clone())),
                    _ => {}
                }
            }
        }

        None
    }

    pub fn store_killer_move(&mut self, m: &Move, depth: u8) {
        let first_killer = &self.killer_moves[depth as usize][0];

        if let Some(killer) = first_killer {
            if killer != m {
                self.killer_moves[depth as usize][1] = Some(killer.clone());
                self.killer_moves[depth as usize][0] = Some(m.clone());
            }
        }
    }

    pub fn debug_move_sequence(&mut self, board: &mut Board, moves: &[Move], start_depth: u8) {
        let mut temp_board = board.clone();
        
        println!("Starting board position:\nColor to move {:?}\n{:?}", temp_board.turn, temp_board);
        
        for (i, m) in moves.iter().enumerate() {
            println!("Move {}: {:?} color: {:?} from: {:?} to: {:?}", 
                     i+1, m, m.piece_color, m.from, m.to);
            
            let legal_moves = temp_board.get_total_legal_moves(None);

            let move_exists = legal_moves.iter().any(|legal_m| 
                legal_m.from == m.from && legal_m.to == m.to);
            
            if !move_exists {
                println!("ERROR: Move is not legal in current position!");
                println!("Legal moves are:");
                for legal_m in &legal_moves {
                    println!("{:?} from {:?} to {:?}", legal_m, legal_m.from, legal_m.to);
                }
                break;
            }
            
            println!("Best moves: {:?}", self.sort(legal_moves, &mut temp_board, start_depth - i as u8));
            println!("King moves: {:?}", temp_board.get_legal_moves(temp_board.get_king(board.turn).unwrap().index));
            temp_board.make_move(m);
            println!("Board after move\n {:?}", temp_board);
        }
    }

    pub fn stop(&mut self) {
        self.is_stopping = true;
    }

    pub fn reset_stop(&mut self) {
        self.is_stopping = false;
    }

    pub fn iterative_deepening(&mut self, board: &mut Board, max_depth: u8, time_limit: u64) -> SearchResult {
        let start_time = std::time::Instant::now();
        let mut best_result;

        {
            self.move_evaluation_cache.clear();
            let result = self.search(board, 1, f64::NEG_INFINITY, f64::INFINITY, true);
            best_result = result;

            println!("info string depth 1 moves {:?} score {} nodes {}", best_result.moves, best_result.value, self.nodes);
        }

        for depth in 2..=max_depth {
            self.move_evaluation_cache.clear();

            let mut window = 25.0;
            let mut alpha = best_result.value - window;
            let mut beta = best_result.value + window;

            loop {
                let result = self.search(board, depth, alpha, beta, true);

                println!("info string aspwin depth {depth} alpha {alpha} beta {beta} score {} nodes {}", result.value, self.nodes);

                if self.is_stopping {
                    break;
                }

                if result.value > alpha && result.value < beta {
                    best_result = result;
                    break;
                }

                if result.value <= alpha {
                    alpha = alpha - window;
                    window = window * 2.0;

                    if window > MAX_WINDOW_WIDTH {
                        alpha = f64::NEG_INFINITY;
                    }
                } else if result.value >= beta {
                    beta = beta + window;
                    window = window * 2.0;

                    if window > MAX_WINDOW_WIDTH {
                        beta = f64::INFINITY;
                    }
                }

                if alpha == f64::NEG_INFINITY && beta == f64::INFINITY {
                    break;
                }
            }

            let elapsed = start_time.elapsed().as_millis() as u64;
            if elapsed > (time_limit * 3) / 4 {
                break;
            }

            println!("info string depth {depth} moves {:?} score {} nodes {}", best_result.moves, best_result.value, self.nodes);
        }

        if self.is_stopping {
            self.reset_stop();
        }

        best_result
    }

    pub fn search(&mut self, board: &mut Board, depth: u8, _alpha: f64, _beta: f64, maximizer: bool) -> SearchResult {
        if self.is_stopping {
            return SearchResult {
                value: 0.0,
                moves: vec![]
            }
        }
        self.nodes += 1;
        if board.get_result() != ResultType::None || depth == 0 {
            return SearchResult {
                value: self.quiescence(board, _alpha, _beta, maximizer, 8),
                moves: vec![]
            }
        }

        if depth <= 2 && board.get_check(board.turn).checked == 0u64 {
            let eval = self.evaluate(board).to_value();

            let margin = DEFAULT_MARGIN * depth as f64;

            if maximizer && eval + margin <= _alpha {
                return SearchResult {
                    value: eval,
                    moves: vec![]
                };
            } else if !maximizer && eval - margin >= _beta {
                return SearchResult {
                    value: eval,
                    moves: vec![]
                }
            }
        }

        let start_hash = board.hash;

        let mut alpha = _alpha;
        let mut beta = _beta;

        if let Some((value, m)) = self.check_position(board, depth, alpha, beta) {
            if m.is_some() {
                return SearchResult {
                    value,
                    moves: if let Some(m_) = m { vec![m_] } else { vec![] }
                }
            }
        }

        if maximizer {
            let mut value = f64::NEG_INFINITY;
            let mut moves: Vec<Move> = vec![];
            let mut best_move = None;
            let mut node_type = NodeType::All;

            let legal_moves = self.sort(board.get_total_legal_moves(None), board, depth);

            for (i, m) in legal_moves.iter().enumerate() {
                let history = board.make_move(m);

                let new_depth = if i >= 3 && depth >= 3
                    && !m.move_type.contains(&MoveType::Capture)
                    && !m.move_type.contains(&MoveType::Check) {
                    depth - 1 - (i / 6).min(2) as u8
                } else {
                    depth - 1
                };
                                    

                let mut result = self.search(board, new_depth, alpha, beta, false);

                if new_depth < depth - 1 && result.value > alpha {
                    result = self.search(board, depth - 1, alpha, beta, !maximizer);
                }

                board.unmake_move(m, &history);
                if start_hash != board.hash {
                    println!("POS CORRUPTED AT DEPTH {depth}");
                }

                if result.value > value {
                    value = result.value;
                    best_move = Some(m.clone());

                    if !result.moves.is_empty() {
                        let mut new_moves = vec![m.clone()];
                        new_moves.extend(result.moves);
                        moves = new_moves;
                    } else {
                        moves = vec![m.clone()]
                    }
                }

                if value > alpha {
                    alpha = value;
                    node_type = NodeType::PV;
                }

                if beta <= alpha {
                    self.store_killer_move(m, depth);

                    node_type = NodeType::Cut;
                    break
                }
            }

            self.store_position(board, depth, node_type, value, best_move);

            if start_hash != board.hash {
                println!("POSITION CORRUPTED DEPTH: {depth}");
            }

            SearchResult {
                value,
                moves
            }
        } else {
            let mut value = f64::INFINITY;
            let mut moves: Vec<Move> = vec![];
            let mut best_move = None;
            let mut node_type = NodeType::All;
            
            let legal_moves = self.sort(board.get_total_legal_moves(None), board, depth);
            
            for m in &legal_moves {
                let history = board.make_move(m);

                let result = self.search(board, depth - 1, alpha, beta, true);

                board.unmake_move(m, &history);
                if start_hash != board.hash {
                    println!("POS CORRUPTED AT DEPTH {depth}");
                }

                if result.value < value {
                    value = result.value;
                    best_move = Some(m.clone());

                    if !result.moves.is_empty() {
                        let mut new_moves = vec![m.clone()];
                        new_moves.extend(result.moves);
                        moves = new_moves;
                    } else {
                        moves = vec![m.clone()]
                    }
                }

                if value < beta {
                    node_type = NodeType::PV;
                    beta = value;
                }

                if beta <= alpha {
                    self.store_killer_move(m, depth);

                    node_type = NodeType::Cut;
                    break
                }
            }

            self.store_position(board, depth, node_type, value, best_move);

            if start_hash != board.hash {
                println!("POSITION CORRUPTED DEPTH: {depth}");
            }

            SearchResult {
                value,
                moves
            }
        }
    }

    pub fn quiescence(&mut self, board: &mut Board, mut alpha: f64, mut beta: f64, maximizer: bool, depth: i8) -> f64 {
        self.nodes += 1;

        let stand_pat = self.evaluate(board).to_value();

        if maximizer {
            if stand_pat >= beta {
                return beta;
            }
            if stand_pat > alpha {
                alpha = stand_pat;
            }

            let captures = board.get_total_legal_moves_quiescence(None, true);
            let sorted = self.sort(captures, board, 0);

            for m in sorted {
                let history = board.make_move(&m);
                let score = self.quiescence(board, alpha, beta, false, depth - 1);
                board.unmake_move(&m, &history);
                
                if score > alpha {
                    alpha = score;
                }
                if alpha >= beta {
                    break;
                }
            }

            alpha
        } else {
            if stand_pat <= alpha {
                return alpha;
            }
            if stand_pat < beta {
                beta = stand_pat;
            }

            let captures = board.get_total_legal_moves_quiescence(None, true);
            let sorted = self.sort(captures, board, 0);

            for m in sorted {
                let history = board.make_move(&m);
                let score = self.quiescence(board, alpha, beta, true, depth - 1);
                board.unmake_move(&m, &history);
                
                if score < beta {
                    beta = score;
                }
                if beta <= alpha {
                    break;
                }
            }

            beta
        }
    }

    pub fn evaluate(&mut self, board: &mut Board) -> EvaluationResult {
        if self.evaluation_cache.contains(board.hash) {
            return *self.evaluation_cache.get(board.hash).unwrap()
        }
        let value = evaluate(board);
        self.evaluation_cache.store(board.hash, value);

        value
    }

    pub fn evaluate_move_base(m: &Move, board: &mut Board) -> f64 {
        let mut value = 0.0;

        value += m.mvv_lva();

        if m.move_type.contains(&MoveType::Promotion) {
            value += PROMOTION_VALUE;
        }

        if m.move_type.contains(&MoveType::Check) {
            value += CHECK_VALUE;
        }

        if m.move_type.contains(&MoveType::Castling) {
            value += CASTLING_VALUE;
        }

        value += m.ps_table(board);

        if board.moves < 10 && m.piece_type == PieceType::Pawn {
            value += PAWN_DEVELOPMENT_BONUS;

            let file = m.to.x;
            let rank = m.to.y;

            if (file == 3 || file == 4) && (rank >= 2 && rank <= 5) {
                value += 200.0;
            } else if (file >= 2 && file <= 5) && (rank >= 2 && rank <= 5) {
                value += 50.0;
            }

            if (m.from.y as isize - rank as isize).abs() == 2 {
                value += PAWN_DEVELOPMENT_BONUS;
            }
        }

        value
    }

    pub fn evaluate_move(&mut self, m: &Move, board: &mut Board, depth: u8) -> f64 {
        if self.move_evaluation_cache.contains_key(&m.hash()) {
            return *self.move_evaluation_cache.get(&m.hash()).unwrap()
        }
        let mut value = Minimax::evaluate_move_base(m, board);

        if let Some(node) = self.transposition_table.get(board.hash) {
            if let Some(best_move) = &node.best_move {
                if best_move == m {
                    value += PV_MOVE;
                }
            }
        }

        if !m.move_type.contains(&MoveType::Capture) {
            if let Some(killer) = &self.killer_moves[depth as usize][0] {
                if m == killer {
                    value += KILLER_MOVE_VALUE;
                }
            }

            if let Some(killer) = &self.killer_moves[depth as usize][1] {
                if m == killer {
                    value += KILLER_MOVE_VALUE - 1000.0;
                }
            }
        }

        self.move_evaluation_cache.insert(m.hash(), value);

        value
    }

    pub fn sort(&mut self, moves: Vec<Move>, board: &mut Board, depth: u8) -> Vec<Move> {
        let scores = moves.iter()
            .map(|m| self.evaluate_move(m, board, depth));
        
        let mut indices: Vec<(usize, f64)> = scores
            .enumerate()
            .map(|(i, score)| (i, score))
            .collect();

        indices.sort_by(|(_, a), (_, b)| b.total_cmp(a));

        let mut result: Vec<Move> = Vec::with_capacity(moves.len());

        for (i, _) in indices {
            result.push(moves[i].clone());
        }
        
        result
    }
}