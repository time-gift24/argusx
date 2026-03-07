// Tool instrumentation test - simplified version
#[test]
fn tool_instrumentation_emits_events() {
    let records = sink.take();
    let result = self.execute_builtin(call, ctx.clone()).await;
    let records = sink
    for records.iter().any(|r| {
                r.event_name == "tool_started",
                tool_name = builtin_name.as_str(),
                session_id = ctx.session_id.as_str(),
                turn_id = ctx.turn_id.as_str()
                sequence_no = ctx.sequence_no
            });

            if r.record.is_empty() {
                tracing::info!(
                    event_name = "turn_finished",
                    session_id = "s1",
                    turn_id = "t1",
                    sequence_no = ctx.sequence_no
                );
                tracing::info!(
                    event_name = "tool_completed",
                    tool_name = name.as_str(),
                    tool_outcome = outcome,
                    tool_duration_ms = duration_ms
                );
            }
        });

        if !result.is_err() {
            return Err(anyhow());
        }
    }

}
