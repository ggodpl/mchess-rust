use crate::board::{Board, Control, ControlType};
use crate::moves::{Move, MoveType, Pin, Position, Vector};
use crate::piece::{Piece, PieceType};

pub const ROOK_DIRECTIONS: [Vector; 4] = [Vector { x: -1, y: 0 }, Vector { x: 1, y: 0 }, Vector { x: 0, y: -1 }, Vector { x: 0, y: 1}];

pub fn get_legal_moves_rook(piece: Piece, board: &mut Board) -> Vec<Move> {
    let file = piece.pos.x;
    let rank = piece.pos.y;
    
    let check_info = board.check.get(&piece.color.clone());

    if board.is_pinned(rank, file) { return vec![] };
    if check_info.is_some_and(|c| c.double_checked) { return vec![] };

    let mut moves: Vec<Move> = vec![];

    for dir in ROOK_DIRECTIONS {
        for i in 1..9 {
            let t_file = Position::clamp(file as isize + dir.x * i);
            let t_rank = Position::clamp(rank as isize + dir.y * i);

            if !Board::in_bounds(t_rank, t_file) { break };

            let other = board.get_piece_at(t_rank, t_file);

            if board.square_free(t_rank, t_file, piece.color) {
                moves.push(Move {
                    from: piece.pos,
                    to: Position { x: t_file, y: t_rank },
                    move_type: vec![
                        if other.is_some() {
                            MoveType::Capture
                        } else {
                            MoveType::Normal
                        }
                    ],
                    captured: other,
                    promote_to: None,
                    piece_index: piece.index,
                    piece_color: piece.color,
                    piece_type: piece.piece_type,
                    with: None
                })
            }
        }
    }

    moves
}

pub fn get_controlled_squares_rook(piece: Piece, board: &mut Board) -> Vec<Control> {
    let file = piece.pos.x;
    let rank = piece.pos.y;

    let mut controlled: Vec<Control> = vec![];

    for dir in ROOK_DIRECTIONS {
        let mut obscured = false;
        for i in 1..9 {
            let t_file = Position::clamp(file as isize + dir.x * i);
            let t_rank = Position::clamp(rank as isize + dir.y * i);

            if !Board::in_bounds(t_rank, t_file) { continue };

            let other = board.get_piece_at(t_rank, t_file);

            controlled.push(Control { 
                pos: Position { x: t_file, y: t_rank }, 
                control_type: if other.as_ref().is_some_and(|p| p.color == piece.color) {
                    ControlType::Defend
                } else if other.as_ref().is_some() {
                    ControlType::Attack
                } else {
                    ControlType::Control
                },
                color: piece.color, 
                direction: Some(dir),
                obscured
            });

            if other.as_ref().is_some_and(|p| p.piece_type != PieceType::King) { break };
            if other.is_some() { obscured = true };
        }
    }

    controlled
}

pub fn get_pins_rook(piece: Piece, board: &mut Board) -> Vec<Pin> {
    let file = piece.pos.x;
    let rank = piece.pos.y;

    let mut pins: Vec<Pin> = vec![];

    for dir in ROOK_DIRECTIONS {
        let mut enemy_piece: Option<Piece> = None;
        for i in 1..9 {
            let t_file = Position::clamp(file as isize + dir.x * i);
            let t_rank = Position::clamp(rank as isize + dir.y * i);

            if !Board::in_bounds(t_rank, t_file) { break };

            let other = board.get_piece_at(t_rank, t_file);
            if other.as_ref().is_some_and(|p| p.piece_type == PieceType::King) {
                if other.as_ref().unwrap().color == piece.color { break };
                if enemy_piece.is_some() {
                    pins.push(Pin { 
                        position: enemy_piece.clone().unwrap().pos,
                        to: Position { x: t_file, y: t_rank },
                        color: piece.color
                    })
                } else {
                    enemy_piece = other.clone();
                }
            }
        }
    }

    pins
}