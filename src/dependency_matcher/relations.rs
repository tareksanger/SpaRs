use crate::{Doc, Result, TokenIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A directed relationship from the already matched left node to the new node.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Relation {
    #[serde(rename = "<")]
    Parent,
    #[serde(rename = ">")]
    Child,
    #[serde(rename = "<<")]
    Ancestor,
    #[serde(rename = ">>")]
    Descendant,
    #[serde(rename = ".")]
    ImmediatelyPrecedes,
    #[serde(rename = ".*")]
    Precedes,
    #[serde(rename = ";")]
    ImmediatelyFollows,
    #[serde(rename = ";*")]
    Follows,
    #[serde(rename = "$+")]
    ImmediateRightSibling,
    #[serde(rename = "$-")]
    ImmediateLeftSibling,
    #[serde(rename = "$++")]
    RightSibling,
    #[serde(rename = "$--")]
    LeftSibling,
    #[serde(rename = ">+")]
    ImmediateRightChild,
    #[serde(rename = ">-")]
    ImmediateLeftChild,
    #[serde(rename = ">++")]
    RightChild,
    #[serde(rename = ">--")]
    LeftChild,
    #[serde(rename = "<+")]
    ImmediateRightParent,
    #[serde(rename = "<-")]
    ImmediateLeftParent,
    #[serde(rename = "<++")]
    RightParent,
    #[serde(rename = "<--")]
    LeftParent,
}

pub(super) struct Graph<'a> {
    doc: &'a Doc,
    pub roots: Vec<usize>,
    bounds: Vec<(usize, usize)>,
    entry: Vec<usize>,
    exit: Vec<usize>,
    depth: Vec<usize>,
    degrees: Vec<usize>,
    cache: HashMap<(usize, Relation), Vec<usize>>,
}
impl<'a> Graph<'a> {
    pub fn new(doc: &'a Doc) -> Result<Self> {
        let n = doc.tokens().len();
        if n > 0 {
            // Builds and validates the shared adjacency index, including cycles.
            doc.token(TokenIndex(0))?.children()?;
        }
        let mut roots = vec![usize::MAX; n];
        let mut path = Vec::new();
        for start in 0..n {
            let mut node = start;
            while roots[node] == usize::MAX {
                path.push(node);
                let head = doc.tokens()[node].head.expect("validated head").0;
                if head == node {
                    roots[node] = node;
                    break;
                }
                node = head;
            }
            let root = roots[node];
            for index in path.drain(..) {
                roots[index] = root;
            }
        }
        let mut bounds = vec![(n, 0); n];
        for (index, &root) in roots.iter().enumerate() {
            bounds[root].0 = bounds[root].0.min(index);
            bounds[root].1 = bounds[root].1.max(index + 1);
        }
        let mut entry = vec![0; n];
        let mut exit = vec![0; n];
        let mut depth = vec![0; n];
        let mut degrees = vec![0; n];
        let mut stack = Vec::new();
        let mut clock = 0;
        for (root, &tree_root) in roots.iter().enumerate() {
            if tree_root != root {
                continue;
            }
            stack.push((root, false));
            while let Some((node, exiting)) = stack.pop() {
                if exiting {
                    exit[node] = clock;
                    continue;
                }
                entry[node] = clock;
                clock += 1;
                stack.push((node, true));
                for child in doc.token(TokenIndex(node))?.children()? {
                    let child = child.index().0;
                    depth[child] = depth[node] + 1;
                    degrees[node] += 1;
                    stack.push((child, false));
                }
            }
        }
        Ok(Self {
            doc,
            roots,
            bounds,
            entry,
            exit,
            depth,
            degrees,
            cache: HashMap::new(),
        })
    }
    /// Use constant-time membership for selective right-hand predicates. Broad
    /// predicates instead enumerate sparse relationships such as direct children.
    pub fn select(
        &mut self,
        left: usize,
        relation: Relation,
        candidates: &[usize],
        output: &mut Vec<usize>,
    ) -> Result<()> {
        use Relation::*;
        let head = self.doc.tokens()[left].head.expect("validated head").0;
        let (start, end) = self.bounds[self.roots[left]];
        let estimate = match relation {
            Ancestor => self.depth[left],
            Descendant => self.exit[left] - self.entry[left] - 1,
            Precedes => end - left - 1,
            Follows => left - start,
            Child | RightChild | LeftChild => self.degrees[left],
            RightSibling | LeftSibling => self.degrees[head],
            _ => 1,
        };
        if candidates.len() <= estimate {
            output.extend(
                candidates
                    .iter()
                    .copied()
                    .filter(|&right| self.contains(left, relation, right)),
            );
        } else {
            let related = self.resolve(left, relation)?;
            output.extend(
                related
                    .iter()
                    .copied()
                    .filter(|i| candidates.binary_search(i).is_ok()),
            );
        }
        Ok(())
    }
    fn contains(&self, left: usize, relation: Relation, right: usize) -> bool {
        use Relation::*;
        let head = self.doc.tokens()[left].head.expect("validated head").0;
        let right_head = self.doc.tokens()[right].head.expect("validated head").0;
        let (start, end) = self.bounds[self.roots[left]];
        let child = right_head == left && right != left;
        // spaCy's sibling operators enumerate head.children: the root itself
        // is excluded as a right operand, but can be a left operand.
        let sibling = right_head == head && right != head;
        match relation {
            Parent => right == head && right != left,
            Child => child,
            Ancestor => {
                right != left
                    && self.entry[right] <= self.entry[left]
                    && self.entry[left] < self.exit[right]
            }
            Descendant => {
                right != left
                    && self.entry[left] <= self.entry[right]
                    && self.entry[right] < self.exit[left]
            }
            ImmediatelyPrecedes => right == left + 1 && right < end,
            Precedes => right > left && right < end,
            ImmediatelyFollows => right.checked_add(1) == Some(left) && right >= start,
            Follows => right < left && right >= start,
            ImmediateRightSibling => sibling && right == left + 1,
            ImmediateLeftSibling => sibling && right.checked_add(1) == Some(left),
            RightSibling => sibling && right > left,
            LeftSibling => sibling && right < left,
            ImmediateRightChild => child && right == left + 1,
            ImmediateLeftChild => child && right.checked_add(1) == Some(left),
            RightChild => child && right > left,
            LeftChild => child && right < left,
            ImmediateRightParent => right == head && right == left + 1,
            ImmediateLeftParent => right == head && right.checked_add(1) == Some(left),
            RightParent => right == head && right > left,
            LeftParent => right == head && right < left,
        }
    }

    pub fn resolve(&mut self, left: usize, relation: Relation) -> Result<&[usize]> {
        if !self.cache.contains_key(&(left, relation)) {
            let token = self.doc.token(TokenIndex(left))?;
            let head = token.annotations().head.expect("validated head").0;
            let (start, end) = self.bounds[self.roots[left]];
            use Relation::*;
            let mut nodes: Vec<usize> = match relation {
                Parent => (head != left).then_some(head).into_iter().collect(),
                Child => token.children()?.map(|t| t.index().0).collect(),
                Ancestor => token.ancestors()?.map(|t| t.index().0).collect(),
                Descendant => token
                    .subtree()?
                    .map(|t| t.index().0)
                    .filter(|&i| i != left)
                    .collect(),
                ImmediatelyPrecedes => (left + 1 < end).then_some(left + 1).into_iter().collect(),
                Precedes => (left + 1..end).collect(),
                ImmediatelyFollows => left
                    .checked_sub(1)
                    .filter(|&i| i >= start)
                    .into_iter()
                    .collect(),
                Follows => (start..left).collect(),
                ImmediateRightSibling | ImmediateRightChild => left
                    .checked_add(1)
                    .filter(|&right| {
                        right < self.roots.len() && self.contains(left, relation, right)
                    })
                    .into_iter()
                    .collect(),
                ImmediateLeftSibling | ImmediateLeftChild => left
                    .checked_sub(1)
                    .filter(|&right| self.contains(left, relation, right))
                    .into_iter()
                    .collect(),
                RightSibling | LeftSibling => self
                    .doc
                    .token(TokenIndex(head))?
                    .children()?
                    .map(|t| t.index().0)
                    .filter(|&i| match relation {
                        ImmediateRightSibling => i == left + 1,
                        ImmediateLeftSibling => i.checked_add(1) == Some(left),
                        RightSibling => i > left,
                        LeftSibling => i < left,
                        _ => unreachable!(),
                    })
                    .collect(),
                RightChild | LeftChild => token
                    .children()?
                    .map(|t| t.index().0)
                    .filter(|&i| match relation {
                        ImmediateRightChild => i == left + 1,
                        ImmediateLeftChild => i.checked_add(1) == Some(left),
                        RightChild => i > left,
                        LeftChild => i < left,
                        _ => unreachable!(),
                    })
                    .collect(),
                ImmediateRightParent => (head == left + 1).then_some(head).into_iter().collect(),
                ImmediateLeftParent => (head.checked_add(1) == Some(left))
                    .then_some(head)
                    .into_iter()
                    .collect(),
                RightParent => (head > left).then_some(head).into_iter().collect(),
                LeftParent => (head < left).then_some(head).into_iter().collect(),
            };
            // Match order comes from token candidates, never traversal order.
            nodes.sort_unstable();
            self.cache.insert((left, relation), nodes);
        }
        Ok(&self.cache[&(left, relation)])
    }
}
