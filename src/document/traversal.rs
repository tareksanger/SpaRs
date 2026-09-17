use super::{Doc, SpanView, TokenIndex, TokenView};
use crate::{Error, Result};

/// Why dependency traversal cannot safely inspect a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DependencyError {
    #[error("dependency head unavailable for token {token:?}")]
    MissingHead { token: TokenIndex },
    #[error("dependency head {head:?} is outside the document for token {token:?}")]
    HeadOutOfBounds { token: TokenIndex, head: TokenIndex },
    #[error("dependency cycle includes token {token:?}")]
    Cycle { token: TokenIndex },
}

#[derive(Debug, Clone)]
pub(crate) struct DependencyIndex {
    offsets: Vec<usize>,
    children: Vec<TokenIndex>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Active,
    Complete,
}

impl DependencyIndex {
    fn build(doc: &Doc) -> std::result::Result<Self, DependencyError> {
        let n = doc.tokens.len();
        let mut offsets = vec![0; n + 1];
        for (i, token) in doc.tokens.iter().enumerate() {
            let head = token.head.ok_or(DependencyError::MissingHead {
                token: TokenIndex(i),
            })?;
            if head.0 >= n {
                return Err(DependencyError::HeadOutOfBounds {
                    token: TokenIndex(i),
                    head,
                });
            }
            if head.0 != i {
                offsets[head.0 + 1] += 1;
            }
        }
        // Each head chain is marked once. A self-headed root is valid; any other
        // edge back into the active chain is a cycle.
        let mut state = vec![VisitState::Unvisited; n];
        for start in 0..n {
            let mut node = start;
            while state[node] == VisitState::Unvisited {
                state[node] = VisitState::Active;
                let head = doc.tokens[node].head.expect("validated head").0;
                if head == node {
                    break;
                }
                node = head;
            }
            if state[node] == VisitState::Active && doc.tokens[node].head != Some(TokenIndex(node))
            {
                return Err(DependencyError::Cycle {
                    token: TokenIndex(node),
                });
            }
            node = start;
            while state[node] == VisitState::Active {
                state[node] = VisitState::Complete;
                node = doc.tokens[node].head.expect("validated head").0;
            }
        }
        for i in 1..offsets.len() {
            offsets[i] += offsets[i - 1];
        }
        let mut children = vec![TokenIndex(0); offsets[n]];
        let mut positions = offsets[..n].to_vec();
        for (i, token) in doc.tokens.iter().enumerate() {
            let head = token.head.expect("validated head").0;
            if head != i {
                children[positions[head]] = TokenIndex(i);
                positions[head] += 1;
            }
        }
        Ok(Self { offsets, children })
    }

    fn children(&self, token: TokenIndex) -> &[TokenIndex] {
        &self.children[self.offsets[token.0]..self.offsets[token.0 + 1]]
    }
}

impl Doc {
    fn dependency_index(&self) -> Result<&DependencyIndex> {
        self.dependency_index
            .get_or_init(|| DependencyIndex::build(self))
            .as_ref()
            .map_err(|error| Error::Dependency(*error))
    }

    /// Borrowed sentence spans, or `None` when sentence boundaries are unavailable.
    pub fn sentence_views(&self) -> Option<impl ExactSizeIterator<Item = SpanView<'_>>> {
        self.sentences.as_ref().map(|sentences| {
            sentences.iter().map(|sentence| SpanView {
                doc: self,
                start: sentence.start,
                end: sentence.end,
            })
        })
    }
}

impl<'a> TokenView<'a> {
    /// Immediate dependents in token order, excluding a root's self-reference.
    /// The first dependency traversal validates and indexes the document in O(n).
    /// Later calls visit only this token's children.
    pub fn children(self) -> Result<Children<'a>> {
        Ok(Children {
            doc: self.doc,
            indices: self.doc.dependency_index()?.children(self.index).iter(),
        })
    }

    /// Heads from the immediate parent to the root, excluding this token.
    /// A root has no ancestors. Missing heads and dependency cycles return errors.
    pub fn ancestors(self) -> Result<Ancestors<'a>> {
        self.doc.dependency_index()?;
        Ok(Ancestors {
            current: Some(self),
        })
    }

    /// Descendants including this token, in spaCy's recursive traversal order:
    /// left child subtrees, this token, then right child subtrees.
    /// Non-projective trees need not produce ascending token indices.
    pub fn subtree(self) -> Result<Subtree<'a>> {
        Ok(Subtree {
            doc: self.doc,
            index: self.doc.dependency_index()?,
            stack: vec![Frame {
                token: self.index,
                child: 0,
                emitted: false,
            }],
        })
    }

    /// The sentence containing this token, without copying its text or tokens.
    /// Returns an error if sentence annotations are unavailable or omit this token.
    pub fn sentence(self) -> Result<SpanView<'a>> {
        let sentences = self
            .doc
            .sentences
            .as_ref()
            .ok_or(Error::MissingAnnotation("sentences"))?;
        let i = sentences.partition_point(|sentence| sentence.end.0 <= self.index.0);
        let sentence = sentences
            .get(i)
            .filter(|sentence| sentence.start.0 <= self.index.0)
            .ok_or(Error::InvalidSentence(self.index))?;
        Ok(SpanView {
            doc: self.doc,
            start: sentence.start,
            end: sentence.end,
        })
    }
}

/// Immediate dependency children in token order.
pub struct Children<'a> {
    doc: &'a Doc,
    indices: std::slice::Iter<'a, TokenIndex>,
}
impl<'a> Iterator for Children<'a> {
    type Item = TokenView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.indices.next().map(|&index| TokenView {
            doc: self.doc,
            index,
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}
impl ExactSizeIterator for Children<'_> {}
impl std::iter::FusedIterator for Children<'_> {}

/// Dependency heads from the immediate parent through the root.
pub struct Ancestors<'a> {
    current: Option<TokenView<'a>>,
}
impl<'a> Iterator for Ancestors<'a> {
    type Item = TokenView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        let head = current.annotations().head.expect("validated head");
        self.current = (head != current.index).then_some(TokenView {
            doc: current.doc,
            index: head,
        });
        self.current
    }
}
impl std::iter::FusedIterator for Ancestors<'_> {}

struct Frame {
    token: TokenIndex,
    child: usize,
    emitted: bool,
}
/// Dependency descendants including the starting token, using an iterative walk.
pub struct Subtree<'a> {
    doc: &'a Doc,
    index: &'a DependencyIndex,
    stack: Vec<Frame>,
}
impl<'a> Iterator for Subtree<'a> {
    type Item = TokenView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let frame = self.stack.last_mut()?;
            if let Some(&child) = self.index.children(frame.token).get(frame.child) {
                if frame.emitted || child.0 < frame.token.0 {
                    frame.child += 1;
                    self.stack.push(Frame {
                        token: child,
                        child: 0,
                        emitted: false,
                    });
                    continue;
                }
            }
            if !frame.emitted {
                frame.emitted = true;
                return Some(TokenView {
                    doc: self.doc,
                    index: frame.token,
                });
            }
            self.stack.pop();
        }
    }
}
impl std::iter::FusedIterator for Subtree<'_> {}

#[cfg(test)]
mod tests;
