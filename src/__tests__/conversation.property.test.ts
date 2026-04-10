/**
 * Feature: openscad-backend, Property 9: Conversation state serialization round-trip
 *
 * For any valid ConversationState object (with an array of messages, optional
 * generated_code, and optional mesh_data), serializing it to JSON and then
 * deserializing the JSON should produce an equivalent ConversationState.
 *
 * Validates: Requirements 7.5
 */
import { describe, it, expect } from 'vitest';
import * as fc from 'fast-check';
import type { Message, MeshData, ConversationState } from '../App';

const messageArb: fc.Arbitrary<Message> = fc.record({
    role: fc.constantFrom('user' as const, 'assistant' as const),
    content: fc.string(),
});

// JSON.stringify(-0) produces "0", so -0 doesn't round-trip through JSON.
// Use a float generator that maps -0 to 0 to stay within the JSON-safe subset.
const jsonSafeFloat = fc
    .float({ noNaN: true, noDefaultInfinity: true })
    .map((v) => (Object.is(v, -0) ? 0 : v));

const meshDataArb: fc.Arbitrary<MeshData> = fc.record({
    vertices: fc.array(jsonSafeFloat),
    normals: fc.array(jsonSafeFloat),
    indices: fc.array(fc.nat()),
});

const conversationStateArb: fc.Arbitrary<ConversationState> = fc.record({
    messages: fc.array(messageArb),
    generated_code: fc.option(fc.string(), { nil: null }),
    mesh_data: fc.option(meshDataArb, { nil: null }),
});

describe('Feature: openscad-backend, Property 9: Conversation state serialization round-trip', () => {
    it('serializing to JSON and deserializing produces an equivalent ConversationState', () => {
        /**
         * Validates: Requirements 7.5
         */
        fc.assert(
            fc.property(conversationStateArb, (state: ConversationState) => {
                const serialized = JSON.stringify(state);
                const deserialized: ConversationState = JSON.parse(serialized);

                expect(deserialized).toEqual(state);
            }),
            { numRuns: 100 },
        );
    });
});
