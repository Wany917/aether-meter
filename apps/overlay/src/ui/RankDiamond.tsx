/** The game's rank marker: a number in an outlined diamond. */
export function RankDiamond({ rank }: { rank: number }) {
  return (
    <span className="rank-diamond">
      <span className="num">{rank}</span>
    </span>
  );
}
