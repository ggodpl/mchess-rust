use mchess::{board::Board, moves::{Move, MoveType}, piece::PieceColor};

use crate::common::{alg, show_mask};

#[test]
fn test_block_positions() {
    let mut board = Board::from_fen("rnbqkbnr/ppp1pppp/3p4/8/2P5/8/PP1PPPPP/RNBQKBNR w KQkq - 0 1");
    let pos = alg("d1");
    let queen = board.get_piece_at(pos.y, pos.x).unwrap();
    board.make_move(&Move {
        from: pos,
        to: alg("a4"),
        move_type: vec![MoveType::Normal],
        captured: None,
        promote_to: None,
        piece_index: queen.index,
        piece_color: queen.color,
        piece_type: queen.piece_type,
        with: None
    });
    let check = board.get_check(PieceColor::Black);

    show_mask(check.block_mask);

    assert_eq!(check.block_positions.clone().unwrap_or(vec![]).len(), 4);
}

#[test]
fn test_block_moves() {
    let mut board = Board::from_fen("rnbqkbnr/ppp1pppp/3p4/8/2P5/8/PP1PPPPP/RNBQKBNR w KQkq - 0 1");
    let pos = alg("d1");
    let queen = board.get_piece_at(pos.y, pos.x).unwrap();
    board.make_move(&Move {
        from: pos,
        to: alg("a4"),
        move_type: vec![MoveType::Normal],
        captured: None,
        promote_to: None,
        piece_index: queen.index,
        piece_color: queen.color,
        piece_type: queen.piece_type,
        with: None
    });

    println!("{:?}", board.get_block_moves(mchess::piece::PieceColor::Black));

    assert_eq!(board.get_block_moves(mchess::piece::PieceColor::Black).len(), 6);
}

#[test]
fn test_king_checked_kiwipete() {
    let mut board = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let moves = board.get_total_legal_moves(None);

    for m in moves {
        if format!("{:?}", m) == "e1f1" {
            board.make_move(&m);
        }
    }

    println!("{:?}", board.get_king(PieceColor::White).unwrap().pos);

    let moves = board.get_total_legal_moves(None);

    for m in moves {
        if format!("{:?}", m) == "h3g2" {
            board.make_move(&m);

            println!("{:?}", board.get_check(PieceColor::Black));

            assert_eq!(board.get_total_legal_moves(None).len(), 4);
        }
    }
}

#[test]
fn test_king_checked_en_passant() {
    let mut board = Board::from_fen("8/8/8/1Ppp3r/1K3p1k/8/4P1P1/1R6 w - c6 0 3");

    show_mask(board.get_check(PieceColor::White).block_mask);
    
    println!("{:?}", board.get_total_legal_moves(None));

    assert_eq!(board.get_total_legal_moves(None).len(), 7);
}